//! Eight stereo lanes, fixed voice pools, sample-accurate sixteenth-note triggering.

use super::audio_file::{Wave, sinc};
#[cfg(test)]
mod tests;

pub const LANES: usize = 8;
pub const STEPS: usize = 64 * 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    #[default]
    OneShot,
    AutoCut,
    Gate,
    Loop,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Lane {
    pub steps: Vec<u8>,
    pub bars: u8,
    pub mode: Mode,
    pub muted: bool,
    pub solo: bool,
    pub gain: f64,
    pub pan: f64,
    pub pitch: i8,
    pub reverse: bool,
    pub start: f64,
    pub end: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub gate: f64,
    pub choke: u8,
    pub send: f64,
}

impl Default for Lane {
    fn default() -> Self {
        Self {
            steps: vec![0; STEPS],
            bars: 1,
            mode: Mode::OneShot,
            muted: false,
            solo: false,
            gain: 0.7,
            pan: 0.0,
            pitch: 0,
            reverse: false,
            start: 0.0,
            end: 1.0,
            attack_ms: 1.0,
            release_ms: 8.0,
            gate: 0.8,
            choke: 0,
            send: 0.0,
        }
    }
}

impl Lane {
    pub fn sanitize(&mut self) {
        // Size is validated before settings enter the callback; this does not allocate.
        self.bars = self.bars.clamp(1, 64);
        self.pitch = self.pitch.clamp(-24, 24);
        self.choke = self.choke.min(8);
        fn finite(x: f64, fallback: f64, lo: f64, hi: f64) -> f64 {
            if x.is_finite() {
                x.clamp(lo, hi)
            } else {
                fallback
            }
        }
        self.gain = finite(self.gain, 0.0, 0.0, 2.0);
        self.pan = finite(self.pan, 0.0, -1.0, 1.0);
        self.start = finite(self.start, 0.0, 0.0, 0.999);
        self.end = finite(self.end, 1.0, self.start + 0.001, 1.0);
        self.attack_ms = finite(self.attack_ms, 1.0, 0.1, 2000.0);
        self.release_ms = finite(self.release_ms, 8.0, 1.0, 3000.0);
        self.gate = finite(self.gate, 0.8, 0.01, 16.0);
        self.send = finite(self.send, 0.0, 0.0, 1.0);
        for v in &mut self.steps {
            *v = (*v).min(127);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Voice {
    active: bool,
    position: f64,
    age: usize,
    velocity: f64,
    release: f64,
    until: f64,
    last: [f64; 2],
}

pub struct Sampler {
    pub waves: [Option<Box<Wave>>; LANES],
    voices: [[Voice; 4]; LANES],
    tails: [[f64; 2]; LANES],
    table: Vec<[f64; 16]>,
    last_step: Option<i64>,
    last_tick: f64,
    playing: bool,
    audition: [bool; LANES],
    gains: [[f64; 2]; LANES],
    sends: [f64; LANES],
}

impl Default for Sampler {
    fn default() -> Self {
        let mut table = Vec::with_capacity(49 * 256);
        for pitch in -24..=24 {
            let cutoff = (1.0 / 2.0f64.powf(f64::from(pitch) / 12.0)).min(1.0) * 0.94;
            for phase in 0..256 {
                let mut weights = std::array::from_fn(|tap| {
                    let d = tap as f64 - 7.0 - f64::from(phase) / 256.0;
                    sinc(d * cutoff) * (0.5 + 0.5 * (d * std::f64::consts::PI / 9.0).cos())
                });
                let norm: f64 = weights.iter().sum();
                for w in &mut weights {
                    *w /= norm;
                }
                table.push(weights);
            }
        }
        Self {
            waves: std::array::from_fn(|_| None),
            voices: [[Voice::default(); 4]; LANES],
            tails: [[0.0; 2]; LANES],
            table,
            last_step: None,
            last_tick: 0.0,
            playing: false,
            audition: [false; LANES],
            gains: [[0.0; 2]; LANES],
            sends: [0.0; LANES],
        }
    }
}

impl Sampler {
    pub fn stop(&mut self) {
        for i in 0..LANES {
            self.cut(i);
        }
        self.last_step = None;
        self.audition = [false; LANES];
        self.playing = false;
    }

    fn cut(&mut self, lane: usize) {
        for voice in &mut self.voices[lane] {
            if voice.active {
                for c in 0..2 {
                    self.tails[lane][c] += voice.last[c];
                }
                voice.active = false;
            }
        }
    }

    pub fn replace(&mut self, lane: usize, wave: Option<Box<Wave>>) -> Option<Box<Wave>> {
        self.cut(lane);
        std::mem::replace(&mut self.waves[lane], wave)
    }

    pub fn audition(&mut self, lane: usize) {
        if lane < LANES {
            self.audition[lane] = true;
        }
    }

    fn trigger(&mut self, lane: usize, velocity: u8, tick: f64, lanes: &[Lane; LANES]) {
        if self.waves[lane].is_none() {
            return;
        }
        let setting = &lanes[lane];
        if setting.choke > 0 {
            for (i, other) in lanes.iter().enumerate() {
                if i != lane && other.choke == setting.choke {
                    self.cut(i);
                }
            }
        }
        if setting.mode != Mode::OneShot {
            self.cut(lane);
        }
        let voices = &mut self.voices[lane];
        let index = voices.iter().position(|v| !v.active).unwrap_or_else(|| {
            voices
                .iter()
                .enumerate()
                .max_by_key(|(_, v)| v.age)
                .unwrap()
                .0
        });
        let old = voices[index];
        if old.active {
            for c in 0..2 {
                self.tails[lane][c] += old.last[c];
            }
        }
        voices[index] = Voice {
            active: true,
            velocity: f64::from(velocity) / 127.0,
            until: tick + 48.0 * setting.gate,
            ..Voice::default()
        };
    }

    /// tick is the start of the current audio block; ticks_per_frame follows the engine tempo.
    pub fn process(
        &mut self,
        lanes: &[Lane; LANES],
        tick: f64,
        ticks_per_frame: f64,
        playing: bool,
        output: &mut [[f64; 64]; 2],
        send: &mut [[f64; 64]; 2],
    ) {
        if (!playing && self.playing)
            || (playing && (tick < self.last_tick || tick - self.last_tick > 12.0))
        {
            self.stop();
        }
        let solo = lanes.iter().any(|l| l.solo);
        let rates: [f64; LANES] =
            std::array::from_fn(|i| 2.0f64.powf(f64::from(lanes[i].pitch) / 12.0));
        for j in 0..64 {
            let now = tick + j as f64 * ticks_per_frame;
            if playing {
                let step = (now / 48.0).floor() as i64;
                if self.last_step != Some(step) {
                    for (i, lane) in lanes.iter().enumerate() {
                        let position = step.rem_euclid(i64::from(lane.bars) * 16) as usize;
                        let velocity = lane.steps.get(position).copied().unwrap_or(0);
                        if velocity > 0 && !lane.muted && (!solo || lane.solo) {
                            self.trigger(i, velocity, now, lanes);
                        }
                    }
                    self.last_step = Some(step);
                }
            }
            for (i, lane) in lanes.iter().enumerate() {
                if self.audition[i] {
                    self.audition[i] = false;
                    self.trigger(i, 127, now, lanes);
                }
                let mut sample = self.tails[i];
                for x in &mut self.tails[i] {
                    *x *= 0.9;
                    if x.abs() < 1e-14 {
                        *x = 0.0;
                    }
                }
                if let Some(wave) = &self.waves[i] {
                    let frames = wave.frames.len();
                    let start = (lane.start * frames as f64) as usize;
                    let end = ((lane.end * frames as f64) as usize).min(frames);
                    let length = end.saturating_sub(start);
                    let rate = rates[i];
                    let seam = (length / 4).min(88) as f64;
                    for voice in &mut self.voices[i] {
                        if !voice.active || length < 2 {
                            continue;
                        }
                        let gate_end = lane.mode == Mode::Gate
                            && (if playing {
                                now >= voice.until
                            } else {
                                voice.age as f64 * ticks_per_frame >= 48.0 * lane.gate
                            });
                        if gate_end {
                            voice.release += 1.0 / (lane.release_ms * 44.1);
                        }
                        if voice.position >= length as f64 {
                            if lane.mode == Mode::Loop {
                                voice.position %= length as f64;
                            } else {
                                voice.active = false;
                                continue;
                            }
                        }
                        let position = if lane.reverse {
                            end as f64 - 1.0 - voice.position
                        } else {
                            start as f64 + voice.position
                        };
                        let center = position.floor() as isize;
                        let phase = ((position - position.floor()) * 256.0) as usize;
                        let weights =
                            &self.table[(lane.pitch + 24) as usize * 256 + phase.min(255)];
                        let mut value = [0.0; 2];
                        for (tap, &weight) in weights.iter().enumerate() {
                            let index = (center + tap as isize - 7)
                                .clamp(start as isize, end as isize - 1)
                                as usize;
                            for (c, v) in value.iter_mut().enumerate() {
                                *v += f64::from(wave.frames[index][c]) * weight;
                            }
                        }
                        if lane.mode == Mode::Loop && seam > 0.0 {
                            let fade = (voice.position / seam).min(1.0)
                                * ((length as f64 - voice.position) / seam).min(1.0);
                            for value in &mut value {
                                *value *= fade;
                            }
                        }
                        let attack = (voice.age as f64 / (lane.attack_ms * 44.1)).min(1.0);
                        let ending = if lane.mode == Mode::Loop {
                            1.0
                        } else {
                            ((length as f64 - voice.position) / (rate * lane.release_ms * 44.1))
                                .min(1.0)
                        };
                        let envelope =
                            attack * ending * (1.0 - voice.release).max(0.0) * voice.velocity;
                        voice.last = value.map(|x| x * envelope);
                        for (x, last) in sample.iter_mut().zip(voice.last) {
                            *x += last;
                        }
                        voice.age += 1;
                        voice.position += rate;
                        if voice.release >= 1.0 {
                            voice.active = false;
                        }
                    }
                }
                let audible = !lane.muted && (!solo || lane.solo);
                let pan = [(1.0 - lane.pan).min(1.0), (1.0 + lane.pan).min(1.0)];
                self.sends[i] += (lane.send - self.sends[i]) * 0.004;
                for c in 0..2 {
                    let target = if audible { lane.gain * pan[c] } else { 0.0 };
                    self.gains[i][c] += (target - self.gains[i][c]) * 0.004;
                    if target == 0.0 && self.gains[i][c].abs() < 1e-10 {
                        self.gains[i][c] = 0.0;
                    }
                    let value = sample[c] * self.gains[i][c];
                    output[c][j] += value;
                    send[c][j] += value * self.sends[i];
                }
            }
        }
        self.last_tick = tick;
        self.playing = playing;
    }
}
