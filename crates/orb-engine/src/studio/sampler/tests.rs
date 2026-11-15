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
    assert!(sampler.voices[0].iter().all(|v| !v.active));
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
