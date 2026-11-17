# ACID RECALL for iPad

The iPad port of ACID RECALL: the same crates, built for `aarch64-apple-ios`, with the
desktop UI (eframe on wgpu and Metal) and the audio output on Core Audio (cpal).

## Prototype 1: sound and screen

What it is: the Graphite rack of the desktop app, scaled to fit the iPad screen in landscape,
playing through the iPad's audio output. Touch drives it like a mouse: one finger at a time.

What changed for iOS:

- `crates/orb-ui/Cargo.toml`: on iOS, eframe without its desktop features (X11, Wayland, the
  clipboard, system fonts, links), and no rfd, which has no iOS backend.
- `crates/orb-ui/src/ios_rfd.rs`: stands in for rfd on iOS. The file pickers (load a sample, load
  an impulse response, import or export a MIDI routing) return nothing, so those buttons do
  nothing yet, and the crash box is not shown (the crash log is still written).
- `crates/orb-ui/src/main.rs`: the wgpu device asks for the GPU's own limits (wgpu's defaults
  want 16 inter-stage shader variables, the iPad Simulator has 15), and a logger writes the
  warnings and errors of eframe, wgpu and winit to `Documents/AcidRecall.log`: on iOS, eframe
  ends the process itself when the renderer cannot start, and that line is the only trace.
- The settings file is `Library/Application Support/OpenRB-338/ui.toml` and the crash logs
  `Library/Logs/OpenRB-338` inside the app's sandbox, as on macOS.
- MIDI is off: the existing backend is WinMM, Windows only.

Songs and samples: the app's Documents folder is shown in the Files app (On My iPad, ACID
RECALL). The in-app file browser opens there.

## Build

On a Mac with Xcode and the pinned toolchain:

```
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
scripts/ios-app.sh sim                     # target/ios/sim/AcidRecall.app, for this Mac's CPU
scripts/ios-simulator-check.sh target/ios/check
scripts/ios-app.sh device                  # target/ios/device/AcidRecall.ipa, unsigned
```

`scripts/ios-simulator-check.sh` installs the app in an iPad Simulator, starts it, and saves
screenshots, the app's output and any crash report, and prints them. It fails when the app is
no longer running after 30 seconds. CI starts the app with `--demo` (no player): the hosted
macOS runners have no audio output, and there the Simulator's Core Audio never answers the
player's output unit, which aborts the app before its window shows. Sound is checked on an
iPad.

CI: `.github/workflows/ios.yml` runs these three steps on an Intel macOS runner (the Apple
silicon runners were too busy to take the job) for every push to
`feat/ipad`, and uploads `AcidRecall-iPad` (the `.ipa`) and `simulator-check` as artifacts of
the run.

## Install on an iPad

The `.ipa` is not signed for a device. A sideloading tool (Sideloadly, AltStore) signs it with
the user's Apple ID when it installs it; with a free Apple ID the app has to be signed again
every seven days. On the iPad: trust the Apple ID in Settings > General > VPN & Device
Management, and turn on Developer Mode in Settings > Privacy & Security.

The App Store is not an option: the GPL-3.0 and the App Store's terms are at odds, and the
built-in kit (`assets/Standard.rbm`) is Propellerhead's material.

## Next

1. Touch layout: one module per screen, swipe between modules, a mixer rail that is always
   visible, controls of at least 44 pt, several knobs at once (multi-touch).
2. MIDI through Core MIDI: USB-C controllers and MIDI Clock, behind the same interface as
   the WinMM backend (`crates/orb-ui/src/midi.rs`).
3. Native pickers (UIDocumentPicker) in place of `ios_rfd`.

The project page is `site/` (`.github/workflows/pages.yml`, GitHub Pages).
