//! Optional Studio instruments and convolution send. Legacy frames have no Studio instance.

pub mod audio_file;
pub mod convolution;
mod fft;
mod mp3;
pub mod sampler;
#[cfg(test)]
mod tests;

use convolution::Convolver;
use sampler::{LANES, Lane, Sampler};

/// One block's logical modulation events, captured around the native sequencer step.
pub struct Modulation {
    pub gates: [bool; 2],
    /// New hits in this block, by machine and native pattern column (accent is unused).
    pub drum_triggers: [[u64; 12]; 2],
    pub kicks: [u64; 2],
    pub audible: [bool; 2],
    pub config: crate::sidechain::Config,
    pub ticks_per_frame: f64,
}

pub(super) fn smooth(value: &mut f64, target: f64, factor: f64) {
    let delta = target - *value;
    if delta.abs() < 1e-12 {
        *value = target;
    } else {
        *value += delta * factor;
    }
}

fn clean(value: f64) -> f64 {
    if value.abs() < 1e-20 { 0.0 } else { value }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub sends: [f64; 4],
    pub enabled: bool,
    pub return_gain: f64,
    pub predelay_ms: f64,
    pub low_cut: f64,
    pub high_cut: f64,
    pub width: f64,
    pub lanes: [Lane; LANES],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lanes: std::array::from_fn(|_| Lane::default()),
            sends: [0.0; 4],
            enabled: true,
            return_gain: 0.25,
            predelay_ms: 18.0,
            low_cut: 140.0,
            high_cut: 10000.0,
            width: 1.0,
        }
    }
}

impl Settings {
    pub fn sanitize(&mut self) {
        fn finite(v: f64, lo: f64, hi: f64) -> f64 {
            if v.is_finite() { v.clamp(lo, hi) } else { lo }
        }
        for lane in &mut self.lanes {
            lane.sanitize();
        }
        self.sends = self.sends.map(|x| finite(x, 0.0, 1.0));
        self.return_gain = finite(self.return_gain, 0.0, 1.0);
        self.predelay_ms = finite(self.predelay_ms, 0.0, 200.0);
        self.low_cut = finite(self.low_cut, 20.0, 2000.0);
        self.high_cut = finite(self.high_cut, 2000.0, 20000.0);
        self.width = finite(self.width, 0.0, 2.0);
    }
}

pub struct Studio {
    pub settings: Box<Settings>,
    pub sampler: Sampler,
    convolver: Box<Convolver>,
    previous: Option<Box<Convolver>>,
    pending: Option<Box<Convolver>>,
    fade: usize,
    send: [[f64; 64]; 2],
    predelay: Vec<[f64; 2]>,
    delay_position: usize,
    smooth_delay: f64,
    high_state: [f64; 2],
    low_state: [f64; 2],
    smooth_sends: [f64; 4],
    smooth_return: f64,
    wet_remaining: usize,
    ducker: crate::sidechain::Ducker,
}

impl Studio {
    /// Prepare on a worker/UI thread, then transfer the box to the player.
    pub fn new(convolver: Box<Convolver>) -> Self {
        Self {
            settings: Box::default(),
            sampler: Sampler::default(),
            convolver,
            previous: None,
            pending: None,
            fade: 0,
            send: [[0.0; 64]; 2],
            predelay: vec![[0.0; 2]; 16384],
            delay_position: 0,
            smooth_delay: 18.0 * 44.1,
            high_state: [0.0; 2],
            low_state: [0.0; 2],
            smooth_sends: [0.0; 4],
            smooth_return: 0.0,
            wet_remaining: 0,
            ducker: crate::sidechain::Ducker::default(),
        }
    }

    pub fn replace_settings(&mut self, mut settings: Box<Settings>) -> Box<Settings> {
        settings.sanitize();
        std::mem::replace(&mut self.settings, settings)
    }

    /// Returns any already-retiring processor to be destroyed on the UI thread.
    pub fn replace_reverb(&mut self, next: Box<Convolver>) -> Option<Box<Convolver>> {
        if self.fade > 0 || self.pending.is_some() {
            return self.pending.replace(next);
        }
        self.fade = 4410;
        self.previous
            .replace(std::mem::replace(&mut self.convolver, next))
    }

    pub fn retired(&mut self) -> Option<Box<Convolver>> {
        if self.fade > 0 {
            return None;
        }
        let old = self.previous.take();
        if let Some(next) = self.pending.take() {
            self.previous = Some(std::mem::replace(&mut self.convolver, next));
            self.fade = 4410;
        }
        old
    }

    pub fn begin(&mut self) {
        self.send = [[0.0; 64]; 2];
    }

    pub fn modulation(&mut self, mut data: Modulation) {
        // Sampler ducking follows the global kick source and release, with its own depth.
        // TB depth controls stay independent from the sampler's opt-in lanes.
        data.config.depth = [65; 2];
        self.ducker.set_config(data.config);
        let duck = self.ducker.gains(data.kicks, data.audible);
        self.sampler.set_modulation(data.gates, duck[0]);
        self.sampler
            .set_drum_modulation(data.drum_triggers, data.ticks_per_frame);
    }

    pub fn channel(&mut self, channel: usize, sample: usize, left: f64, right: f64) {
        let target = if self.settings.enabled {
            self.settings.sends[channel]
        } else {
            0.0
        };
        smooth(&mut self.smooth_sends[channel], target, 0.002);
        self.send[0][sample] += left * self.smooth_sends[channel];
        self.send[1][sample] += right * self.smooth_sends[channel];
    }

    pub fn process(
        &mut self,
        tick: f64,
        ticks_per_frame: f64,
        playing: bool,
        left: &mut [f32; 64],
        right: &mut [f32; 64],
    ) {
        let mut dry = [[0.0; 64]; 2];
        self.sampler.process(
            &self.settings.lanes,
            tick,
            ticks_per_frame,
            playing,
            &mut dry,
            &mut self.send,
        );
        let delay = self.settings.predelay_ms * 44.1;
        let hp = (-std::f64::consts::TAU * self.settings.low_cut / 44100.0).exp();
        let lp = (-std::f64::consts::TAU * self.settings.high_cut / 44100.0).exp();
        for j in 0..64 {
            self.predelay[self.delay_position] = [self.send[0][j], self.send[1][j]];
            smooth(&mut self.smooth_delay, delay, 0.001);
            let whole = self.smooth_delay.floor() as usize;
            let fraction = self.smooth_delay - whole as f64;
            let a = self.predelay[(self.delay_position + 16384 - whole) & 16383];
            let b = self.predelay[(self.delay_position + 16383 - whole) & 16383];
            let input: [f64; 2] =
                std::array::from_fn(|c| a[c] * (1.0 - fraction) + b[c] * fraction);
            self.delay_position = (self.delay_position + 1) & 16383;
            for (c, &sample) in input.iter().enumerate() {
                self.high_state[c] = clean(hp * self.high_state[c] + (1.0 - hp) * sample);
                let high = sample - self.high_state[c];
                self.low_state[c] = clean(lp * self.low_state[c] + (1.0 - lp) * high);
                self.send[c][j] = self.low_state[c];
            }
        }
        let mut wet = [[0.0; 64]; 2];
        if self.send.iter().flatten().any(|&x| x != 0.0) {
            // Include the maximum supported IR and partition padding before sleeping.
            self.wet_remaining = 6 * 44100 + 4096;
        }
        if self.wet_remaining > 0 {
            self.convolver.process(&self.send, &mut wet);
            self.wet_remaining = self.wet_remaining.saturating_sub(64);
        }
        if self.fade > 0 {
            let mut old = [[0.0; 64]; 2];
            if let Some(previous) = &mut self.previous {
                previous.process(&self.send, &mut old);
            }
            for j in 0..64 {
                let mix = self.fade as f64 / 4410.0;
                for c in 0..2 {
                    wet[c][j] = wet[c][j] * (1.0 - mix) + old[c][j] * mix;
                }
                self.fade = self.fade.saturating_sub(1);
            }
        }
        let target = if self.settings.enabled {
            self.settings.return_gain
        } else {
            0.0
        };
        for j in 0..64 {
            smooth(&mut self.smooth_return, target, 0.002);
            let mid = (wet[0][j] + wet[1][j]) * 0.5;
            let side = (wet[0][j] - wet[1][j]) * 0.5 * self.settings.width;
            let add = [
                (mid + side) * self.smooth_return + dry[0][j],
                (mid - side) * self.smooth_return + dry[1][j],
            ];
            if add[0] != 0.0 {
                left[j] = (f64::from(left[j]) + add[0]) as f32;
            }
            if add[1] != 0.0 {
                right[j] = (f64::from(right[j]) + add[1]) as f32;
            }
        }
    }
}
