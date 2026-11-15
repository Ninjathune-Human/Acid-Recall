//! Studio controls use the existing controller router and MIDI pickup behavior.

use crate::{app::App, model::Event};

pub fn controls() -> Vec<(String, String)> {
    let mut controls = vec![("studio.return".into(), "SPACE / Return".into()),
        ("studio.predelay".into(), "SPACE / Pre-delay".into()),
        ("studio.width".into(), "SPACE / Stereo width".into()),
        ("studio.low_cut".into(), "SPACE / Low cut".into()),
        ("studio.high_cut".into(), "SPACE / High cut".into())];
    for (i, label) in ["TB-303 A", "TB-303 B", "TR-808", "TR-909"].iter().enumerate() {
        controls.push((format!("studio.send{}", i + 1), format!("SPACE / {label} send")));
    }
    for i in 1..=8 {
        for name in ["level", "pan", "pitch", "send"] {
            controls.push((format!("sampler{i}.{name}"), format!("Sampler / Lane {i} / {name}")));
        }
    }
    controls
}

impl App {
    pub(crate) fn studio_midi(&mut self) {
        let before = self.studio.settings.clone();
        for event in &self.panel.events {
            let Event::Set { id, value } = event else { continue };
            let v = f64::from((*value).clamp(0, 127)) / 127.0;
            let s = &mut self.studio.settings;
            match id.as_str() {
                "studio.return" => s.return_gain = v,
                "studio.predelay" => s.predelay_ms = v * 200.0,
                "studio.width" => s.width = v * 2.0,
                "studio.low_cut" => s.low_cut = 20.0 + v * 1980.0,
                "studio.high_cut" => s.high_cut = 2000.0 + v * 18000.0,
                _ => {
                    if let Some(index) = id.strip_prefix("studio.send").and_then(|s| s.parse::<usize>().ok()) {
                        if (1..=4).contains(&index) { s.sends[index - 1] = v; }
                    } else if let Some((lane, name)) = id.strip_prefix("sampler").and_then(|s| s.split_once('.'))
                        && let Ok(index) = lane.parse::<usize>() && (1..=8).contains(&index) {
                        let lane = &mut s.lanes[index - 1];
                        match name { "level" => lane.gain = v * 2.0,
                            "pan" => lane.pan = v * 2.0 - 1.0,
                            "pitch" => lane.pitch = (v * 48.0 - 24.0).round() as i8,
                            "send" => lane.send = v, _ => {} }
                    }
                }
            }
        }
        if self.studio.settings != before { self.studio.changed(); self.doc.modified = true; }
        self.studio_mirror();
    }

    pub(crate) fn studio_mirror(&mut self) {
        let s = &self.studio.settings;
        for (id, _) in controls() {
            let v = match id.as_str() {
                "studio.return" => s.return_gain,
                "studio.predelay" => s.predelay_ms / 200.0,
                "studio.width" => s.width / 2.0,
                "studio.low_cut" => (s.low_cut - 20.0) / 1980.0,
                "studio.high_cut" => (s.high_cut - 2000.0) / 18000.0,
                _ if id.starts_with("studio.send") => {
                    let i = id.trim_start_matches("studio.send").parse::<usize>().unwrap() - 1;
                    s.sends[i]
                }
                _ => {
                    let (lane, name) = id.trim_start_matches("sampler").split_once('.').unwrap();
                    let lane = &s.lanes[lane.parse::<usize>().unwrap() - 1];
                    match name { "level" => lane.gain / 2.0, "pan" => (lane.pan + 1.0) / 2.0,
                        "pitch" => (f64::from(lane.pitch) + 24.0) / 48.0, _ => lane.send }
                }
            };
            self.panel.adopt(&id, (v * 127.0).round() as i32);
        }
    }
}
