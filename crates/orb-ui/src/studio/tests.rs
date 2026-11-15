use super::*;
use eframe::egui;
use orb_engine::tables::Tables;
use orb_player::{Audio, CALL_FRAMES};
use crate::{link::Link, model::Panel};

fn setup() -> (App, Audio, Context) {
    let samples = Some(rbm::standard::builtin_samples());
    let (link, mut audio) = Link::new(Arc::new(Tables::new()), samples.clone());
    let mut app = App::with_link(Panel::new(), link, None);
    app.samples = samples;
    app.new_now();
    audio.render_i16(&mut vec![0; CALL_FRAMES * 4]);
    app.link.as_mut().unwrap().sync(&mut app.panel);
    let ir = Arc::new(Wave { frames: vec![[0.1; 2]; 64] });
    app.studio_command(Command::Studio(Box::new(Studio::new(Box::new(Convolver::new(&ir.frames).unwrap())))));
    app.studio.ir = Some(ir);
    app.studio.ready = true;
    app.studio.content = true;
    audio.render_i16(&mut vec![0; CALL_FRAMES * 4]);
    app.link.as_mut().unwrap().sync(&mut app.panel);
    let ctx = Context::default();
    (app, audio, ctx)
}

fn finish(app: &mut App, audio: &mut Audio, ctx: &Context) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.studio.busy() {
        audio.render_i16(&mut vec![0; CALL_FRAMES * 2]);
        app.studio_poll(ctx);
        assert!(std::time::Instant::now() < deadline, "Studio operation timed out");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn save_and_open_restore_embedded_sample_last_step_and_mix_settings() {
    let (mut app, mut audio, ctx) = setup();
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("session.orb338");
    app.studio.settings.lanes[7].bars = 64;
    app.studio.settings.lanes[7].steps[1023] = 117;
    app.studio.waves[7] = Some(Arc::new(Wave { frames: vec![[0.2, -0.1]; 1000] }));
    app.settings.sidechain.depth = [21, 48];
    app.settings.closed_hat_level = Some(42);
    app.doc.modified = true;
    app.studio_save(&path, &ctx);
    finish(&mut app, &mut audio, &ctx);
    assert!(app.studio.notice.is_none(), "{:?}", app.studio.notice);
    assert!(path.is_file());
    assert!(!app.doc.modified);
    app.studio.settings.lanes[7].steps[1023] = 0;
    app.studio_open(&path, &ctx);
    assert!(app.studio.opening);
    finish(&mut app, &mut audio, &ctx);
    assert_eq!(app.studio.settings.lanes[7].steps[1023], 117);
    assert_eq!(app.studio.waves[7].as_ref().unwrap().frames.len(), 1000);
    assert_eq!(app.settings.sidechain.depth, [21, 48]);
    assert_eq!(app.settings.closed_hat_level, Some(42));
    assert_eq!(app.doc.path.as_ref(), Some(&path));
    assert!(!app.doc.modified);
}

#[test]
fn async_save_does_not_clear_edits_made_after_its_snapshot() {
    let (mut app, mut audio, ctx) = setup();
    let folder = tempfile::tempdir().unwrap();
    app.doc.modified = true;
    app.studio_save(&folder.path().join("session.orb338"), &ctx);
    app.settings.sidechain.depth[0] = 13;
    finish(&mut app, &mut audio, &ctx);
    assert!(app.doc.modified);
    assert!(app.after_save.is_none());
}

#[test]
fn successful_save_runs_deferred_new_and_rejected_save_clears_the_deferred_action() {
    let (mut app, mut audio, ctx) = setup();
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("session.orb338");
    app.doc.modified = true;
    app.after_save = Some(crate::menu::Then::New);
    app.studio_save(&path, &ctx);
    finish(&mut app, &mut audio, &ctx);
    assert!(path.is_file());
    assert!(!app.studio.content && !app.doc.modified);
    let (_tx, rx) = mpsc::sync_channel(1);
    app.studio.worker = Some(rx);
    app.after_save = Some(crate::menu::Then::Quit);
    app.studio_save(&path, &ctx);
    assert!(app.after_save.is_none());
}

#[test]
fn invalid_open_preserves_dirty_document_and_existing_studio() {
    let (mut app, mut audio, ctx) = setup();
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("broken.orb338");
    std::fs::write(&path, b"not a project").unwrap();
    app.studio.settings.return_gain = 0.73;
    app.doc.modified = true;
    let settlement = app.settlement;
    app.studio_open(&path, &ctx);
    finish(&mut app, &mut audio, &ctx);
    assert!(app.studio.notice.is_some());
    assert_eq!(app.studio.settings.return_gain, 0.73);
    assert!(app.doc.modified);
    assert_eq!(app.settlement, settlement);
}

#[test]
fn midi_routes_independently_to_machine_sends_and_sampler_zero() {
    let (mut app, mut audio, ctx) = setup();
    app.panel.set_knob("studio.send4", 1000);
    app.panel.set_knob("sampler8.level", 0);
    app.studio_midi();
    app.studio_poll(&ctx);
    audio.render_i16(&mut vec![0; CALL_FRAMES * 4]);
    let studio = audio.current_mut().frame.studio.as_ref().unwrap();
    assert_eq!(studio.settings.sends, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(studio.settings.lanes[7].gain, 0.0);
    assert_eq!(studio.settings.lanes[6].gain, 0.7);
}

#[test]
fn new_editors_render_in_modern_rack_without_an_audio_device() {
    let ctx = Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut app = App::new(Panel::new(), None);
    app.studio.modern = true;
    app.studio.ready = true;
    app.studio.sampler_open = true;
    app.studio.reverb_open = true;
    let mut output = ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))), ..Default::default() }, |ui| app.show(ui));
    output.textures_delta.clear();
    assert!(app.studio.sampler_open && app.studio.reverb_open);
}
