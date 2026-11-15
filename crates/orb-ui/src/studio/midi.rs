//! Studio controls share MIDI pickup and per-device calibration.
use orb_engine::studio::{Settings, sampler::{Lane, Mode}};
use orb_player::Command;
use crate::{app::App, model::Event};

pub fn controls() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, label) in [("enabled", "Enabled"), ("return", "Return"),
        ("predelay", "Pre-delay"), ("width", "Stereo width"), ("low_cut", "Low cut"),
        ("high_cut", "High cut")] {
        out.push((format!("studio.{name}"), format!("SPACE / {label}")));
    }
    for (i, label) in ["TB-303 A", "TB-303 B", "TR-808", "TR-909"].iter().enumerate() {
        out.push((format!("studio.send{}", i + 1), format!("SPACE / {label} send")));
    }
    for i in 1..=8 {
        for name in ["level", "pan", "pitch", "send", "start", "end", "attack", "release",
            "gate", "mode", "reverse", "mute", "solo", "audition", "choke", "gater_source",
            "gater_depth", "gater_attack_ms", "gater_release_ms", "sidechain"] {
            out.push((format!("sampler{i}.{name}"),
                format!("Sampler / Lane {i} / {}", name.replace('_', " "))));
        }
    }
    out
}

pub fn is_action(id: &str) -> bool {
    id.starts_with("sampler") && id.ends_with(".audition")
}

pub fn is_boolean(id: &str) -> bool {
    id == "studio.enabled" || id.starts_with("sampler")
        && [".reverse", ".mute", ".solo", ".sidechain"].iter().any(|s| id.ends_with(s))
}

fn lane_field(id: &str) -> Option<(usize, &str)> {
    let (lane, name) = id.strip_prefix("sampler")?.split_once('.')?;
    let i = lane.parse::<usize>().ok()?.checked_sub(1)?;
    (i < 8).then_some((i, name))
}

fn mode(v: f64) -> Mode {
    match (v * 3.0).round() as u8 {
        0 => Mode::OneShot, 1 => Mode::AutoCut, 2 => Mode::Gate, _ => Mode::Loop,
    }
}

fn lane_value(lane: &Lane, name: &str) -> f64 {
    match name {
        "level" => lane.gain / 2.0,
        "pan" => (lane.pan + 1.0) / 2.0,
        "pitch" => (f64::from(lane.pitch) + 24.0) / 48.0,
        "send" => lane.send,
        "start" => lane.start / (1.0 - 1e-9),
        "end" => (lane.end - 1e-9) / (1.0 - 1e-9),
        "attack" => (lane.attack_ms - 0.1) / 1999.9,
        "release" => (lane.release_ms - 1.0) / 2999.0,
        "gate" => (lane.gate - 0.01) / 15.99,
        "mode" => match lane.mode {
            Mode::OneShot => 0.0, Mode::AutoCut => 1.0 / 3.0,
            Mode::Gate => 2.0 / 3.0, Mode::Loop => 1.0,
        },
        "choke" => f64::from(lane.choke) / 8.0,
        "gater_source" => f64::from(lane.gater_source) / 2.0,
        "gater_depth" => lane.gater_depth,
        "gater_attack_ms" => (lane.gater_attack_ms - 0.1) / 99.9,
        "gater_release_ms" => (lane.gater_release_ms - 1.0) / 499.0,
        "reverse" => f64::from(u8::from(lane.reverse)),
        "mute" => f64::from(u8::from(lane.muted)),
        "solo" => f64::from(u8::from(lane.solo)),
        "sidechain" => f64::from(u8::from(lane.sidechain)),
        _ => 0.0,
    }
}

fn value(settings: &Settings, id: &str) -> f64 {
    match id {
        "studio.enabled" => f64::from(u8::from(settings.enabled)),
        "studio.return" => settings.return_gain,
        "studio.predelay" => settings.predelay_ms / 200.0,
        "studio.width" => settings.width / 2.0,
        "studio.low_cut" => (settings.low_cut - 20.0) / 1980.0,
        "studio.high_cut" => (settings.high_cut - 2000.0) / 18000.0,
        _ => {
            if let Some(i) = id.strip_prefix("studio.send")
                .and_then(|s| s.parse::<usize>().ok()).and_then(|n| n.checked_sub(1)) {
                settings.sends.get(i).copied().unwrap_or(0.0)
            } else if let Some((i, name)) = lane_field(id) {
                lane_value(&settings.lanes[i], name)
            } else { 0.0 }
        }
    }
}

pub fn defaults() -> Vec<(String, u8)> {
    let settings = Settings::default();
    controls().into_iter().map(|(id, _)| {
        let cc = (value(&settings, &id).clamp(0.0, 1.0) * 127.0).round() as u8;
        (id, cc)
    }).collect()
}

impl App {
    pub(crate) fn studio_midi(&mut self) {
        let before = self.studio.settings.clone();
        let mut auditions = Vec::new();
        for event in &self.panel.events {
            if let Event::Press { id } = event {
                if let Some((i, "audition")) = lane_field(id) { auditions.push(i); }
                continue;
            }
            let Event::Set { id, value } = event else { continue };
            let raw = (*value).clamp(0, 127);
            let v = f64::from(raw) / 127.0;
            let s = &mut self.studio.settings;
            match id.as_str() {
                "studio.enabled" => s.enabled = raw >= 64,
                "studio.return" => s.return_gain = v,
                "studio.predelay" => s.predelay_ms = v * 200.0,
                "studio.width" => s.width = v * 2.0,
                "studio.low_cut" => s.low_cut = 20.0 + v * 1980.0,
                "studio.high_cut" => s.high_cut = 2000.0 + v * 18000.0,
                _ => {
                    if let Some(index) = id.strip_prefix("studio.send")
                        .and_then(|s| s.parse::<usize>().ok()) {
                        if (1..=4).contains(&index) { s.sends[index - 1] = v; }
                    } else if let Some((i, name)) = lane_field(id) {
                        let lane = &mut s.lanes[i];
                        match name {
                            "level" => lane.gain = v * 2.0,
                            "pan" => lane.pan = v * 2.0 - 1.0,
                            "pitch" => lane.pitch = (v * 48.0 - 24.0).round() as i8,
                            "send" => lane.send = v,
                            "start" => {
                                lane.start = v * (1.0 - 1e-9);
                                lane.end = lane.end.max(lane.start + 1e-9);
                            }
                            "end" => lane.end = (1e-9 + v * (1.0 - 1e-9)).max(lane.start + 1e-9),
                            "attack" => lane.attack_ms = 0.1 + v * 1999.9,
                            "release" => lane.release_ms = 1.0 + v * 2999.0,
                            "gate" => lane.gate = 0.01 + v * 15.99,
                            "mode" => lane.mode = mode(v),
                            "choke" => lane.choke = (v * 8.0).round() as u8,
                            "gater_source" => lane.gater_source = (v * 2.0).round() as u8,
                            "gater_depth" => lane.gater_depth = v,
                            "gater_attack_ms" => lane.gater_attack_ms = 0.1 + v * 99.9,
                            "gater_release_ms" => lane.gater_release_ms = 1.0 + v * 499.0,
                            "reverse" => lane.reverse = raw >= 64,
                            "mute" => lane.muted = raw >= 64,
                            "solo" => lane.solo = raw >= 64,
                            "sidechain" => lane.sidechain = raw >= 64,
                            _ => {}
                        }
                    }
                }
            }
        }
        if self.studio.settings != before {
            self.studio.changed();
            self.doc.modified = true;
        }
        if !auditions.is_empty() {
            self.studio_poll(&self.studio.context.clone());
            for lane in auditions { self.studio_command(Command::StudioAudition(lane)); }
        }
        self.studio_mirror();
    }

    pub(crate) fn studio_mirror(&mut self) {
        let s = &self.studio.settings;
        for (id, _) in controls() {
            if is_action(&id) { continue; }
            if let Some((i, name)) = lane_field(&id)
                && let Some(raw) = self.panel.value(&id) {
                let v = f64::from(raw) / 127.0;
                let lane = &s.lanes[i];
                // Quantization and trim constraints must not re-arm pickup on our own edits.
                let same = match name {
                    "pitch" => (v * 48.0 - 24.0).round() as i8 == lane.pitch,
                    "mode" => mode(v) == lane.mode,
                    "choke" => (v * 8.0).round() as u8 == lane.choke,
                    "gater_source" => (v * 2.0).round() as u8 == lane.gater_source,
                    "end" => ((1e-9 + v * (1.0 - 1e-9)).max(lane.start + 1e-9)
                        - lane.end).abs() < 1e-12,
                    _ => false,
                };
                if same { continue; }
            }
            self.panel.adopt(&id, (value(s, &id).clamp(0.0, 1.0) * 127.0).round() as i32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{midi::{Binding, Cc, Config, Kind, Router, kind}, model::Panel};

    fn move_cc(app: &mut App, router: &mut Router, id: &str, value: u8, pickup: bool) {
        let config = Config { pickup, bindings: vec![Binding {
            input: 0, channel: 1, controller: 74, target: id.into(),
        }], ..Config::default() };
        router.apply(Cc { channel: 1, controller: 74, value }, &config, &mut app.panel, 0.0);
        app.studio_midi();
        app.panel.events.clear();
    }

    #[test]
    fn all_studio_controls_are_registered_with_correct_midi_behavior() {
        assert_eq!(controls().len(), 170);
        for (id, _) in controls() {
            assert!(kind(&id).is_some(), "{id}");
            assert!(Panel::new().value(&id).is_some(), "{id}");
        }
        assert_eq!(kind("sampler8.audition"), Some(Kind::Button));
        assert_eq!(kind("sampler1.mute"), Some(Kind::Switch));
        assert_eq!(kind("studio.enabled"), Some(Kind::Switch));
    }

    #[test]
    fn selectors_keep_pickup_latched_through_every_quantization_boundary() {
        for id in ["sampler1.pitch", "sampler1.mode", "sampler1.choke", "sampler1.gater_source"] {
            let mut app = App::new(Panel::new(), None);
            let mut router = Router::default();
            let current = app.panel.value(id).unwrap();
            for value in (0..=current).rev().chain(1..=127) {
                move_cc(&mut app, &mut router, id, value, true);
                assert_eq!(app.panel.value(id), Some(value), "{id}: CC {value}");
            }
            let lane = &app.studio.settings.lanes[0];
            match id {
                "sampler1.pitch" => assert_eq!(lane.pitch, 24),
                "sampler1.mode" => assert_eq!(lane.mode, Mode::Loop),
                "sampler1.choke" => assert_eq!(lane.choke, 8),
                _ => assert_eq!(lane.gater_source, 2),
            }
        }
    }

    #[test]
    fn envelope_gater_and_boolean_endpoints_reach_the_settings() {
        let mut app = App::new(Panel::new(), None);
        let mut router = Router::default();
        for name in ["attack", "release", "gate", "gater_depth", "gater_attack_ms",
            "gater_release_ms", "reverse", "mute", "solo", "sidechain"] {
            move_cc(&mut app, &mut router, &format!("sampler8.{name}"), 127, false);
        }
        let lane = &app.studio.settings.lanes[7];
        assert_eq!((lane.attack_ms, lane.release_ms, lane.gate), (2000.0, 3000.0, 16.0));
        assert_eq!((lane.gater_depth, lane.gater_attack_ms, lane.gater_release_ms), (1.0, 100.0, 500.0));
        assert!(lane.reverse && lane.muted && lane.solo && lane.sidechain);
        for name in ["attack", "release", "gate", "gater_depth", "gater_attack_ms",
            "gater_release_ms", "reverse", "mute", "solo", "sidechain"] {
            move_cc(&mut app, &mut router, &format!("sampler8.{name}"), 0, true);
        }
        let lane = &app.studio.settings.lanes[7];
        assert_eq!(lane.gater_depth, 0.0);
        assert!(!lane.reverse && !lane.muted && !lane.solo && !lane.sidechain);
        move_cc(&mut app, &mut router, "studio.enabled", 0, true);
        assert!(!app.studio.settings.enabled);
    }

    #[test]
    fn moving_trim_end_below_start_keeps_valid_bounds_and_pickup_continues() {
        let mut app = App::new(Panel::new(), None);
        let mut router = Router::default();
        move_cc(&mut app, &mut router, "sampler1.start", 100, false);
        for value in (0..=127).rev() {
            move_cc(&mut app, &mut router, "sampler1.end", value, true);
            assert_eq!(app.panel.value("sampler1.end"), Some(value));
            let lane = &app.studio.settings.lanes[0];
            assert!(lane.end > lane.start && lane.end <= 1.0);
        }
    }
}
