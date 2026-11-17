//! `OpenRB-338`: the panel window, wired to the real-time player (task-045). The binary has this
//! name (crate `orb-ui`) because the macOS Dock and Activity Monitor show the executable's name.
//!
//! ```text
//! OpenRB-338 [SONG]
//! ```
//! `--help` shows only this. The options below are for developers, tests and screenshots
//! (task-137): hidden from `--help` and the usage text, but they all work (docs/building.md,
//! "OpenRB-338 developer options"):
//!
//! ```text
//! --demo  --size WxH  --ppp S  --browse open|save|export|mod  --about  --info
//! --screenshot OUT.png  --play
//! ```
//!
//! `-h` is `--help`; an argument `-psn_...` (Finder on old macOS) is ignored; a bad option prints
//! the usage and exits 2.
//!
//! With a song it opens that song, with the mod it names (looked up in the Mods folder of the
//! settings, `ORB_MOD_DIR`, or the default places of `orb render`); without one the panel starts on
//! a new empty song. The menu bar (task-047, docs/ui_spec.md section 18) opens, saves and exports
//! songs, chooses the mod and sets the Vintage flag and the keyboard mode. The drum machines play
//! the built-in kit "Standard ReBirth", the standard mod orb embeds (`assets/Standard.rbm`), or
//! the song's mod laid over it. `--demo`
//! starts the panel in a lively state with no player (a picture). The window starts maximized;
//! `--size` opens it, not maximized, with that inner size of the panel in points (624x510 is
//! the native size; the window is as high as the menu bar more, for screenshots), `--ppp`
//! overrides the pixel density. `--screenshot` saves the window to a PNG after a few frames and
//! quits. Ctrl/Cmd+O opens the in-app file browser (task-046) to open a song, Ctrl/Cmd+S saves;
//! `--about` starts with the About window open (for screenshots, task-115), `--info` with File >
//! Info's Song Information window (task-117). `--play` presses Play once the song is open (CPU
//! measurements, task-153).
//! `--browse MODE` starts with the browser open for a mode. The browser's folders are kept in the
//! settings file (`OpenRB-338/ui.toml` in the configuration folder, or `$ORB_UI_CONFIG`).

// The Windows release build is a GUI-subsystem program: no console window opens behind the
// panel (task-111). `orb` is not affected (this attribute is in the UI binary only). A debug
// build keeps the console, for `dbg!` and panics.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use orb_engine::tables::Tables;
use orb_ui::app::{App, Shot};
use orb_ui::browser::Mode;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::link::Link;
use orb_ui::menu::MENU_H;
use orb_ui::model::Panel;
use orb_ui::session;
use orb_ui::settings::Settings;

/// The visible help: only the song. The developer options are hidden (task-137).
const USAGE: &str = "usage: ACID-RECALL [SONG]\n\
                     SONG is a ReBirth song (.rbs) or a Studio project (.orb338) to open";

/// The command line, parsed.
#[derive(Debug, PartialEq)]
struct Args {
    studio_view: Option<String>,
    demo: bool,
    about: bool,
    info: bool,
    /// `--play`: press Play once the song is open (CPU measurements, task-153).
    play: bool,
    size: [f32; 2],
    sized: bool,
    ppp: Option<f32>,
    shot: Option<PathBuf>,
    song: Option<PathBuf>,
    browse: Option<Mode>,
    debug_panic: Option<DebugPanic>,
}

/// Where `--debug-panic` panics on purpose (task-157): tests of the crash hook.
#[derive(Debug, PartialEq, Clone, Copy)]
enum DebugPanic {
    /// On the main thread at the start, before any window (runs with no display).
    Start,
    /// On the main thread, once the window is up.
    Window,
    /// On another thread, half a second after the window is up.
    Thread,
}

/// What the command line asks instead of running the app.
#[derive(Debug, PartialEq)]
enum Exit {
    /// `--help`: print the usage, exit 0.
    Help,
    /// A bad option or value: print the usage, exit 2.
    Bad,
}

/// Parses the arguments after the program name. `-psn_...` (Finder on old macOS) is ignored.
fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args, Exit> {
    let mut r = Args {
        studio_view: None,
        demo: false,
        about: false,
        info: false,
        play: false,
        size: [PANEL_W, PANEL_H],
        sized: false,
        ppp: None,
        shot: None,
        song: None,
        browse: None,
        debug_panic: None,
    };
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--help" | "-h" => return Err(Exit::Help),
            "--demo" => r.demo = true,
            "--about" => r.about = true,
            "--info" => r.info = true,
            "--play" => r.play = true,
            "--studio-view" => {
                r.studio_view = Some(match args.next().as_deref() {
                    Some("sampler") => "sampler".into(),
                    Some("space") => "space".into(),
                    _ => return Err(Exit::Bad),
                });
            }
            "--debug-panic" => r.debug_panic = Some(DebugPanic::Start),
            "--debug-panic-window" => r.debug_panic = Some(DebugPanic::Window),
            "--debug-panic-thread" => r.debug_panic = Some(DebugPanic::Thread),
            "--size" => {
                let v = args.next().ok_or(Exit::Bad)?;
                let (w, h) = v.split_once('x').ok_or(Exit::Bad)?;
                r.size = [
                    w.parse().map_err(|_| Exit::Bad)?,
                    h.parse().map_err(|_| Exit::Bad)?,
                ];
                r.sized = true;
            }
            "--ppp" => r.ppp = Some(args.next().and_then(|v| v.parse().ok()).ok_or(Exit::Bad)?),
            "--screenshot" => r.shot = Some(args.next().ok_or(Exit::Bad)?.into()),
            "--browse" => {
                r.browse = Some(match args.next().as_deref() {
                    Some("open") => Mode::OpenSong,
                    Some("save") => Mode::SaveSong,
                    Some("export") => Mode::Export,
                    Some("mod") => Mode::ChooseMod,
                    _ => return Err(Exit::Bad),
                })
            }
            s if s.starts_with("-psn_") => {}
            s if !s.starts_with('-') && r.song.is_none() => r.song = Some(s.into()),
            _ => return Err(Exit::Bad),
        }
    }
    Ok(r)
}

fn usage(e: Exit) -> ! {
    match e {
        Exit::Help => {
            println!("{USAGE}");
            std::process::exit(0);
        }
        Exit::Bad => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}
/// Gives a GUI-subsystem process the console of the program that started it (task-111).
///
/// Such a process has no console, so `eprintln!` (the usage text, a song that does not open, the
/// audio error) goes nowhere. When it is started from `cmd` or PowerShell, this attaches to that
/// console and points the standard output and error at it, unless they were redirected (a file
/// or a pipe stays as it is). Started from the Start menu or Explorer there is no parent
/// console and nothing happens. What stays lost: `cmd` does not wait for a GUI program, so the
/// prompt comes back at once and the text appears over it; `start /wait OpenRB-338.exe` waits.
#[cfg(all(windows, not(debug_assertions)))]
fn attach_parent_console() {
    use std::fs::OpenOptions;
    use std::os::windows::io::IntoRawHandle;

    type Handle = *mut core::ffi::c_void;
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn GetStdHandle(std_handle: u32) -> Handle;
        fn SetStdHandle(std_handle: u32, handle: Handle) -> i32;
    }
    // SAFETY: plain Win32 calls; the handle given to SetStdHandle is leaked on purpose, it must
    // live as long as the process.
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }
        for id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let h = GetStdHandle(id);
            // none (null) or invalid (-1): not redirected, so use the console
            if (h.is_null() || h as isize == -1)
                && let Ok(f) = OpenOptions::new().write(true).open("CONOUT$")
            {
                SetStdHandle(id, f.into_raw_handle());
            }
        }
    }
}

/// The iPad's log (docs/ipad.md): the warnings and errors of eframe, wgpu and winit go to stderr
/// and to `Documents/AcidRecall.log`, which the Files app shows. On iOS, eframe ends the process
/// itself when the window or the renderer cannot start, so `main` never returns its error: the
/// log line eframe writes first is the only trace.
#[cfg(target_os = "ios")]
mod ios_log {
    use std::io::Write as _;

    struct Logger;

    impl log::Log for Logger {
        fn enabled(&self, m: &log::Metadata) -> bool {
            m.level() <= log::Level::Warn
        }

        fn log(&self, r: &log::Record) {
            if !self.enabled(r.metadata()) {
                return;
            }
            let line = format!("{} {}: {}\n", r.level(), r.target(), r.args());
            eprint!("{line}");
            let path = orb_ui::settings::documents_dir().join("AcidRecall.log");
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path);
            if let Ok(mut f) = file {
                let _ = f.write_all(line.as_bytes());
            }
        }

        fn flush(&self) {}
    }

    pub fn install() {
        if log::set_logger(&Logger).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
    }
}

/// The iPad's renderer (docs/ipad.md): the device asks for the limits the GPU has rather than
/// wgpu's defaults, which the iPad Simulator does not reach (`max_inter_stage_shader_variables`
/// 15, not 16: wgpu refuses the device and eframe ends the app).
#[cfg(target_os = "ios")]
fn ios_wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};

    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.device_descriptor = Arc::new(|adapter| eframe::wgpu::DeviceDescriptor {
        label: Some("egui wgpu device"),
        required_limits: adapter.limits(),
        ..Default::default()
    });
    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

/// The browser (docs/web.md): the app in the page's canvas `#acid-recall`. No command line and
/// no files: the new empty song with the built-in kit. The sound goes through Web Audio (cpal),
/// which the page's script resumes on the first touch or key, as browsers start it suspended.
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let _ = eframe::WebLogger::init(log::LevelFilter::Warn);
    let samples = Some(rbm::standard::builtin_samples());
    let (link, audio) = Link::new(Arc::new(Tables::new()), samples.clone());
    let mut start_errors: Vec<String> = Vec::new();
    let mut audio_health = None;
    match orb_player::output::start(audio) {
        Ok(s) => {
            audio_health = Some(s.health());
            // the page never ends the program: the stream lives as long as the page
            std::mem::forget(s);
        }
        Err(e) => {
            log::error!("audio output: {e}");
            start_errors.push(format!("Audio output: {e}; the panel runs without sound"));
        }
    }
    let canvas = eframe::web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("acid-recall"))
        .and_then(|e| e.dyn_into::<eframe::web_sys::HtmlCanvasElement>().ok())
        .expect("the page has a canvas #acid-recall");
    wasm_bindgen_futures::spawn_local(async move {
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
                    let mut app = App::with_link(Panel::new(), link, None);
                    app.samples = samples;
                    app.audio_health = audio_health;
                    app.menu = true;
                    app.studio.modern = true;
                    app.enable_logo_anim();
                    app.settings = Settings::load();
                    app.action(|app| {
                        app.start(None);
                        for e in start_errors {
                            app.fail(e);
                        }
                    });
                    Ok(Box::new(Fitted(app)))
                }),
            )
            .await;
        if let Err(e) = started {
            log::error!("the app did not start: {e:?}");
        }
    });
}

/// The browser's app, zoomed to fit the window (docs/web.md): the whole rack and the menu bar
/// show without scrolling at any window size, an iPad's included, and again after a rotation.
/// Never zoomed in past 1.
#[cfg(target_arch = "wasm32")]
struct Fitted(App);

#[cfg(target_arch = "wasm32")]
impl eframe::App for Fitted {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let zoom = ctx.zoom_factor();
        // the window in CSS pixels: points at zoom 1
        let window = ctx.content_rect().size() * zoom;
        let [w, h] = orb_ui::RACK_MIN;
        let fit = (window.x / w).min(window.y / (h + MENU_H)).min(1.0);
        if (fit - zoom).abs() > 0.005 {
            ctx.set_zoom_factor(fit);
        }
        self.0.ui(ui, frame);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    #[cfg(all(windows, not(debug_assertions)))]
    attach_parent_console();
    // before anything that can panic: a panic writes a crash log and shows a box (task-157)
    orb_ui::crash::install(orb_ui::about::VERSION);
    #[cfg(target_os = "ios")]
    ios_log::install();
    let Args {
        studio_view,
        demo,
        about,
        info,
        play,
        size,
        sized,
        ppp,
        shot,
        song,
        browse,
        debug_panic,
    } = parse_args(std::env::args().skip(1)).unwrap_or_else(|e| usage(e));
    if debug_panic == Some(DebugPanic::Start) {
        panic!("--debug-panic: a deliberate panic");
    }

    let panel = if demo { Panel::demo() } else { Panel::new() };
    let mut link = None;
    let mut app_samples = None;
    let mut startup = None;
    // errors of the start, shown in the message box once the app is up (task-130)
    let mut start_errors: Vec<String> = Vec::new();
    // keeps the audio stream alive for the whole run
    let mut _stream = None;
    let mut audio_health = None;
    if !demo {
        let samples = Some(rbm::standard::builtin_samples());
        app_samples = samples.clone();
        let (l, audio) = Link::new(Arc::new(Tables::new()), samples.clone());
        // The song named on the command line is opened by the app (with its mod).
        if let Some(path) = &song
            && !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("orb338"))
        {
            match session::open_song(path) {
                Ok(o) => startup = Some((path.clone(), o)),
                // the app starts on the new song, with the error in its message box
                Err(e) => {
                    eprintln!("OpenRB-338: {e}");
                    start_errors.push(e);
                }
            }
        }
        match orb_player::output::start(audio) {
            Ok(s) => {
                audio_health = Some(s.health());
                _stream = Some(s);
            }
            Err(e) => {
                eprintln!("OpenRB-338: audio output: {e}; the panel runs without sound");
                // the one failure at the launch is an error the user expects to hear of: the
                // box. Later audio errors repeat: `App::notify`.
                start_errors.push(format!("Audio output: {e}; the panel runs without sound"));
            }
        }
        link = Some(l);
    }

    // The window starts maximized; an explicit --size opens at that size instead.
    let viewport = egui::ViewportBuilder::default()
        .with_title(orb_ui::about::NAME)
        .with_app_id("OpenRB-338")
        .with_icon(Arc::new(orb_ui::art::app_icon(256)))
        // the menu bar is above the panel
        .with_inner_size([size[0], size[1] + MENU_H])
        // Keep native rack controls readable when the window is restored from maximized.
        .with_min_inner_size([1000.0, 650.0 + MENU_H])
        .with_maximized(!sized);
    let options = eframe::NativeOptions {
        viewport,
        #[cfg(target_os = "ios")]
        wgpu_options: ios_wgpu_options(),
        ..Default::default()
    };
    eframe::run_native(
        orb_ui::about::NAME,
        options,
        Box::new(move |cc| {
            // always dark, whatever the system theme (task-127; `App::show` keeps it)
            cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
            // ORB_GPU_INFO=1 prints the adapter wgpu chose (scripts/appimage-smoke.sh)
            if std::env::var_os("ORB_GPU_INFO").is_some_and(|v| v == "1")
                && let Some(rs) = &cc.wgpu_render_state
            {
                let i = rs.adapter.get_info();
                eprintln!("gpu: {:?} {:?} {}", i.backend, i.device_type, i.name);
            }
            if let Some(ppp) = ppp {
                cc.egui_ctx.set_pixels_per_point(ppp);
            }
            match debug_panic {
                Some(DebugPanic::Window) => panic!("--debug-panic-window: a deliberate panic"),
                Some(DebugPanic::Thread) => {
                    std::thread::Builder::new()
                        .name("debug-panic".into())
                        .spawn(|| {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            panic!("--debug-panic-thread: a deliberate panic");
                        })
                        .expect("spawn the debug thread");
                }
                Some(DebugPanic::Start) | None => {}
            }
            let shot = shot.map(Shot::new);
            let mut app = match link {
                Some(l) => App::with_link(panel, l, shot),
                None => App::new(panel, shot),
            };
            app.samples = app_samples;
            app.audio_health = audio_health;
            app.menu = true;
            app.studio.modern = true;
            if demo {
                let frames = (0..88200)
                    .map(|i| {
                        let t = i as f32 / 44100.0;
                        let envelope = (t * 5.0).sin().abs();
                        let value = (t * 900.0).sin() * envelope * 0.5;
                        [value, value]
                    })
                    .collect();
                app.studio.waves[0] =
                    Some(Arc::new(orb_engine::studio::audio_file::Wave { frames }));
                app.studio.names[0] = "Demo waveform".into();
                for step in [0, 4, 8, 12] {
                    app.studio.settings.lanes[0].steps[step] = 100;
                }
            }
            app.enable_logo_anim();
            app.settings = Settings::load();
            // the song (or the new empty song) and its mod's startup sound; nothing in the demo,
            // which has no player
            // (the errors of the start are one box)
            app.action(|app| {
                if !demo {
                    app.start(startup);
                }
                if let Some(path) = &song
                    && path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("orb338"))
                {
                    app.open_now(path);
                }
                for e in start_errors {
                    app.fail(e);
                }
            });
            if let Some(view) = studio_view {
                app.studio.sampler_open = view == "sampler";
                app.studio.reverb_open = view == "space";
            }
            if let Some(mode) = browse {
                app.open_browser(mode);
            }
            if about {
                app.show_about();
            }
            if info {
                app.show_info();
            }
            if play {
                app.panel.press("transport.play", 0.0);
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(a: &[&str]) -> Result<Args, Exit> {
        parse_args(a.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_arguments_is_the_new_song() {
        let a = parse(&[]).unwrap();
        assert_eq!(a.song, None);
        assert!(!a.demo && !a.about && !a.info && !a.sized && !a.play);
        assert_eq!(a.size, [PANEL_W, PANEL_H]);
    }

    #[test]
    fn help_is_exit_0_and_a_bad_option_is_exit_2() {
        assert_eq!(parse(&["--help"]), Err(Exit::Help));
        assert_eq!(parse(&["-h"]), Err(Exit::Help));
        assert_eq!(parse(&["song.rbs", "--help"]), Err(Exit::Help));
        for bad in [
            &["--nope"][..],
            &["-x"],
            &["--size"],
            &["--size", "10"],
            &["--size", "axb"],
            &["--ppp", "x"],
            &["--screenshot"],
            &["--browse", "other"],
            &["a.rbs", "b.rbs"],
        ] {
            assert_eq!(parse(bad), Err(Exit::Bad), "{bad:?}");
        }
    }

    #[test]
    fn the_hidden_options_still_parse() {
        let a = parse(&[
            "--demo",
            "--size",
            "300x200",
            "--ppp",
            "2",
            "--browse",
            "export",
            "--about",
            "--info",
            "--screenshot",
            "out.png",
            "--play",
        ])
        .unwrap();
        assert!(a.demo && a.about && a.info && a.sized && a.play);
        assert_eq!(a.size, [300.0, 200.0]);
        assert_eq!(a.ppp, Some(2.0));
        assert_eq!(a.browse, Some(Mode::Export));
        assert_eq!(a.shot, Some("out.png".into()));
    }

    #[test]
    fn the_song_is_a_path_with_spaces_and_psn_is_ignored() {
        let a = parse(&["-psn_0_12345", "/my songs/a b.rbs"]).unwrap();
        assert_eq!(a.song, Some("/my songs/a b.rbs".into()));
        let a = parse(&["-psn_0_12345"]).unwrap();
        assert_eq!(a.song, None);
    }

    #[test]
    fn the_usage_shows_only_the_song() {
        assert!(USAGE.contains("[SONG]"));
        for hidden in [
            "--demo",
            "--size",
            "--ppp",
            "--browse",
            "--screenshot",
            "--about",
            "--info",
            "--play",
        ] {
            assert!(!USAGE.contains(hidden), "{hidden}");
        }
    }
}
