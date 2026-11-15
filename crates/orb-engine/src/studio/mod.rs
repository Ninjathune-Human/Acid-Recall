//! Optional Studio instruments and convolution send. Legacy frames have no Studio instance.

pub mod audio_file;
pub mod convolution;
mod fft;
pub mod sampler;

use convolution::Convolver;
use sampler::{LANES, Lane, Sampler};

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub lanes: [Lane; LANES],
    pub sends: [f64; 4],
    pub enabled: bool,
    pub return_gain: f64,
    pub predelay_ms: f64,
    pub low_cut: f64,
    pub high_cut: f64,
    pub width: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { lanes: std::array::from_fn(|_| Lane::default()), sends: [0.0; 4],
            enabled: true, return_gain: 0.25, predelay_ms: 18.0,
            low_cut: 140.0, high_cut: 10000.0, width: 1.0 }
    }
}

impl Settings {
    pub fn sanitize(&mut self) {
        fn finite(v: f64, lo: f64, hi: f64) -> f64 {
            if v.is_finite() { v.clamp(lo, hi) } else { lo }
        }
        for lane in &mut self.lanes { lane.sanitize(); }
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
    fade: usize,
    send: [[f64; 64]; 2],
    predelay: Vec<[f64; 2]>,
    delay_position: usize,
    high_state: [f64; 2],
    low_state: [f64; 2],
    smooth_sends: [f64; 4],
    smooth_return: f64,
}

impl Studio {
    /// Prepare on a worker/UI thread, then transfer the box to the player.
    pub fn new(convolver: Box<Convolver>) -> Self {
        Self { settings: Box::default(), sampler: Sampler::default(), convolver,
            previous: None, fade: 0, send: [[0.0; 64]; 2],
            predelay: vec![[0.0; 2]; 16384], delay_position: 0,
            high_state: [0.0; 2], low_state: [0.0; 2], smooth_sends: [0.0; 4],
            smooth_return: 0.0 }
    }

    pub fn replace_settings(&mut self, mut settings: Box<Settings>) -> Box<Settings> {
        settings.sanitize();
        std::mem::replace(&mut self.settings, settings)
    }

    /// Returns any already-retiring processor to be destroyed on the UI thread.
    pub fn replace_reverb(&mut self, next: Box<Convolver>) -> Option<Box<Convolver>> {
        self.fade = 4410;
        self.previous.replace(std::mem::replace(&mut self.convolver, next))
    }

    pub fn retired(&mut self) -> Option<Box<Convolver>> {
        if self.fade == 0 { self.previous.take() } else { None }
    }

    pub fn begin(&mut self) { self.send = [[0.0; 64]; 2]; }

    pub fn channel(&mut self, channel: usize, sample: usize, left: f64, right: f64) {
        let target = if self.settings.enabled { self.settings.sends[channel] } else { 0.0 };
        self.smooth_sends[channel] += (target - self.smooth_sends[channel]) * 0.002;
        self.send[0][sample] += left * self.smooth_sends[channel];
        self.send[1][sample] += right * self.smooth_sends[channel];
    }

    pub fn process(&mut self, tick: f64, ticks_per_frame: f64, playing: bool,
        left: &mut [f32; 64], right: &mut [f32; 64]) {
        let mut dry = [[0.0; 64]; 2];
        self.sampler.process(&self.settings.lanes, tick, ticks_per_frame, playing,
            &mut dry, &mut self.send);
        let delay = (self.settings.predelay_ms * 44.1).round() as usize;
        let hp = (-std::f64::consts::TAU * self.settings.low_cut / 44100.0).exp();
        let lp = (-std::f64::consts::TAU * self.settings.high_cut / 44100.0).exp();
        for j in 0..64 {
            self.predelay[self.delay_position] = [self.send[0][j], self.send[1][j]];
            let input = self.predelay[(self.delay_position + 16384 - delay) & 16383];
            self.delay_position = (self.delay_position + 1) & 16383;
            for (c, &sample) in input.iter().enumerate() {
                self.high_state[c] = hp * self.high_state[c] + (1.0 - hp) * sample;
                let high = sample - self.high_state[c];
                self.low_state[c] = lp * self.low_state[c] + (1.0 - lp) * high;
                self.send[c][j] = self.low_state[c];
            }
        }
        let mut wet = [[0.0; 64]; 2];
        self.convolver.process(&self.send, &mut wet);
        if self.fade > 0 {
            let mut old = [[0.0; 64]; 2];
            if let Some(previous) = &mut self.previous { previous.process(&self.send, &mut old); }
            for j in 0..64 {
                let mix = self.fade as f64 / 4410.0;
                for c in 0..2 { wet[c][j] = wet[c][j] * (1.0 - mix) + old[c][j] * mix; }
                self.fade = self.fade.saturating_sub(1);
            }
        }
        let target = if self.settings.enabled { self.settings.return_gain } else { 0.0 };
        for j in 0..64 {
            self.smooth_return += (target - self.smooth_return) * 0.002;
            let mid = (wet[0][j] + wet[1][j]) * 0.5;
            let side = (wet[0][j] - wet[1][j]) * 0.5 * self.settings.width;
            let add = [(mid + side) * self.smooth_return + dry[0][j],
                (mid - side) * self.smooth_return + dry[1][j]];
            if add[0] != 0.0 { left[j] = (f64::from(left[j]) + add[0]) as f32; }
            if add[1] != 0.0 { right[j] = (f64::from(right[j]) + add[1]) as f32; }
        }
    }
}
