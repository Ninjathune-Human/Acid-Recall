use super::*;

fn setup(mode: Mode) -> (Sampler, [Lane; LANES]) {
    let mut sampler = Sampler::default();
    sampler.replace(
        0,
        Some(Box::new(Wave {
            frames: vec![[0.2, -0.1]; 50000],
        })),
    );
    let mut lanes = std::array::from_fn(|_| Lane::default());
    lanes[0].mode = mode;
    lanes[0].attack_ms = 0.1;
    lanes[0].release_ms = 1.0;
    (sampler, lanes)
}

fn block(sampler: &mut Sampler, lanes: &[Lane; LANES], tick: f64, playing: bool) -> [[f64; 64]; 2] {
    let mut out = [[0.0; 64]; 2];
    sampler.process(lanes, tick, 0.01, playing, &mut out, &mut [[0.0; 64]; 2]);
    out
}

#[test]
fn older_lane_settings_default_new_modulation_to_bypass() {
    let mut old = serde_json::to_value(Lane::default()).unwrap();
    let map = old.as_object_mut().unwrap();
    for key in ["gater_source", "gater_depth", "gater_attack_ms", "gater_release_ms", "sidechain"] {
        map.remove(key);
    }
    let lane: Lane = serde_json::from_value(old).unwrap();
    assert_eq!(lane.gater_source, 0);
    assert!(!lane.sidechain);
    assert_eq!(lane.steps.len(), STEPS);
}

#[test]
fn auto_cut_is_monophonic_one_shot_overlaps_and_stop_releases_every_mode() {
    for mode in [Mode::OneShot, Mode::AutoCut, Mode::Gate, Mode::Loop] {
        let (mut sampler, lanes) = setup(mode);
        for _ in 0..3 {
            sampler.audition(0);
            block(&mut sampler, &lanes, 0.0, false);
        }
        let voices = sampler.voices[0].iter().filter(|v| v.active).count();
        assert_eq!(voices, if mode == Mode::OneShot { 3 } else { 1 });
        sampler.stop();
        for _ in 0..12 {
            block(&mut sampler, &lanes, 0.0, false);
        }
        let out = block(&mut sampler, &lanes, 0.0, false);
        assert!(out.iter().flatten().all(|&x| x == 0.0));
    }
}

#[test]
fn bar_64_last_step_wraps_without_retriggering_idle_blocks() {
    let (mut sampler, mut lanes) = setup(Mode::OneShot);
    lanes[0].bars = 64;
    lanes[0].steps[1023] = 127;
    block(&mut sampler, &lanes, 1023.0 * 48.0, true);
    assert_eq!(sampler.voices[0].iter().filter(|v| v.active).count(), 1);
    block(&mut sampler, &lanes, 1023.0 * 48.0 + 0.64, true);
    assert_eq!(sampler.voices[0].iter().filter(|v| v.active).count(), 1);
    block(&mut sampler, &lanes, 0.0, true);
    assert_eq!(sampler.voices[0].iter().filter(|v| v.active).count(), 1);
}

#[test]
fn one_shots_survive_wraps_and_excess_retriggers_without_stealing() {
    let (mut sampler, lanes) = setup(Mode::OneShot);
    for _ in 0..4 {
        sampler.audition(0);
        block(&mut sampler, &lanes, 0.0, false);
    }
    let ages = sampler.voices[0].map(|v| v.age);
    sampler.audition(0);
    block(&mut sampler, &lanes, 0.0, false);
    assert_eq!(sampler.voices[0].map(|v| v.age), ages.map(|age| age + 64));
    block(&mut sampler, &lanes, 500.0, true);
    block(&mut sampler, &lanes, 0.0, true);
    assert_eq!(sampler.voices[0].iter().filter(|v| v.active).count(), 4);
    for n in 0..800 {
        block(&mut sampler, &lanes, n as f64 * 0.64, true);
    }
    assert!(sampler.voices[0].iter().all(|v| !v.active));
}

#[test]
fn gate_duration_counts_played_ticks_across_a_transport_wrap() {
    let (mut sampler, mut lanes) = setup(Mode::Gate);
    lanes[0].gate = 0.1;
    sampler.audition(0);
    block(&mut sampler, &lanes, 767.0, true);
    for step in 0..20 {
        block(&mut sampler, &lanes, step as f64 * 0.64, true);
    }
    assert!(sampler.voices[0].iter().all(|v| !v.active));
}

#[test]
fn gater_follows_selected_tb_and_sidechain_is_opt_in() {
    let (mut sampler, mut lanes) = setup(Mode::Loop);
    lanes[0].gater_source = 2;
    lanes[0].gater_release_ms = 1.0;
    sampler.audition(0);
    sampler.set_modulation([true, false], [1.0; 64]);
    for _ in 0..80 {
        block(&mut sampler, &lanes, 0.0, false);
    }
    assert!(sampler.modulation[0] < 1e-10);
    sampler.set_modulation([false, true], [0.0; 64]);
    for _ in 0..100 {
        block(&mut sampler, &lanes, 0.0, false);
    }
    assert!(sampler.modulation[0] > 0.99, "unchecked sidechain must bypass");
    lanes[0].sidechain = true;
    let mut out = [[0.0; 64]; 2];
    for _ in 0..80 {
        out = block(&mut sampler, &lanes, 0.0, false);
    }
    assert!(out.iter().flatten().all(|v| v.abs() < 1e-10));
    assert!(sampler.modulation[0] > 0.99, "sidechain must not alter the gater envelope");
    lanes[0].gater_source = 0;
    lanes[0].sidechain = false;
    for _ in 0..100 {
        block(&mut sampler, &lanes, 0.0, false);
    }
    assert_eq!(sampler.modulation[0], 1.0);
}

#[test]
fn choke_groups_cut_other_lanes_and_unload_tail_cannot_return_on_reload() {
    let (mut sampler, mut lanes) = setup(Mode::OneShot);
    sampler.replace(
        1,
        Some(Box::new(Wave {
            frames: vec![[0.1; 2]; 10000],
        })),
    );
    lanes[0].choke = 3;
    lanes[1].choke = 3;
    sampler.audition(0);
    block(&mut sampler, &lanes, 0.0, false);
    sampler.audition(1);
    block(&mut sampler, &lanes, 0.0, false);
    assert!(sampler.voices[0].iter().all(|v| !v.active));
    assert!(sampler.voices[1].iter().any(|v| v.active));
    sampler.replace(1, None);
    for _ in 0..12 {
        block(&mut sampler, &lanes, 0.0, false);
    }
    assert_eq!(sampler.tails[1], [0.0; 2]);
    sampler.replace(
        1,
        Some(Box::new(Wave {
            frames: vec![[0.1; 2]; 10000],
        })),
    );
    let out = block(&mut sampler, &lanes, 0.0, false);
    assert!(out.iter().flatten().all(|&x| x == 0.0));
}

#[test]
fn voice_trigger_occurs_at_the_sixteenth_crossing_inside_a_block() {
    let (mut sampler, mut lanes) = setup(Mode::OneShot);
    lanes[0].steps[1] = 127;
    let out = block(&mut sampler, &lanes, 47.7, true);
    assert!(out.iter().all(|c| c[..30].iter().all(|&x| x == 0.0)));
    assert!(out[0][32..].iter().any(|&x| x > 0.0));
    assert!(out[1][32..].iter().any(|&x| x < 0.0));
}

#[test]
fn loop_seam_preserves_duration_and_mute_reaches_exact_silence() {
    let (mut sampler, mut lanes) = setup(Mode::Loop);
    sampler.replace(
        0,
        Some(Box::new(Wave {
            frames: vec![[0.2; 2]; 1000],
        })),
    );
    sampler.audition(0);
    for _ in 0..30 {
        block(&mut sampler, &lanes, 0.0, false);
    }
    let voice = sampler.voices[0].iter().find(|v| v.active).unwrap();
    assert_eq!(voice.position, 920.0);
    lanes[0].muted = true;
    for _ in 0..100 {
        block(&mut sampler, &lanes, 0.0, false);
    }
    assert!(
        block(&mut sampler, &lanes, 0.0, false)
            .iter()
            .flatten()
            .all(|&x| x == 0.0)
    );
}

#[test]
fn native_pitch_reads_pcm_without_resampling() {
    let (mut sampler, mut lanes) = setup(Mode::OneShot);
    lanes[0].gain = 1.0;
    sampler.gains[0] = [1.0; 2];
    sampler.audition(0);
    block(&mut sampler, &lanes, 0.0, false);
    let out = block(&mut sampler, &lanes, 0.0, false);
    assert!(out[0].iter().all(|&x| x == f64::from(0.2f32)));
    assert!(out[1].iter().all(|&x| x == f64::from(-0.1f32)));
}
