# ACID RECALL in the browser

The same crates built for `wasm32-unknown-unknown`: the desktop UI (eframe on wgpu, WebGPU or
WebGL 2) draws into a canvas, and the engine plays through Web Audio (cpal's Web Audio host).
It is published with the project page: `site/app/` on GitHub Pages
(`.github/workflows/pages.yml`), and opens in Safari on an iPad as on a desktop browser.

## What changed for the browser

- `crates/orb-ui/src/main.rs`: a `main` for wasm32 that starts the app in the page's canvas
  `#acid-recall` on the new empty song with the built-in kit, and logs to the browser console.
- `crates/orb-ui/Cargo.toml`: eframe with wgpu and the built-in fonts only, and no rfd, whose
  blocking dialogs do not exist in a browser: `rfd_stub` stands in, as on iOS (docs/ipad.md).
- `crates/orb-player`: cpal's `wasm-bindgen` feature (the Web Audio host), and `web_time`'s
  `Instant` (std's panics in a browser). `crates/orb-ui` takes `web_time`'s `SystemTime` for the
  same reason (the pattern generator's seed is read when the app starts).
- `crates/orb-cli`: no libvorbis on wasm32 (it is C); the Ogg export returns an error there.
- `web/index.html`: the canvas, and a script that resumes the page's audio contexts on the
  first touch, click or key (browsers start Web Audio suspended, and only a call made inside
  such an event may resume it). On iOS 17 and later it also asks for the playback audio
  session, so the sound plays with the ring/silent switch on silent.

## Limits

- No MIDI: Safari has no Web MIDI, and the MIDI backend is WinMM (Windows) anyway.
- No files: the file pickers do nothing, settings are not saved, and the file browser has no
  folder to show.
- Higher latency than the desktop app: cpal's Web Audio host plays 2048-frame buffers.
- The audio runs on the page's main thread, with the UI: a heavy frame can cause a dropout.

## Build

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version VERSION --locked   # VERSION: wasm-bindgen in Cargo.lock
scripts/web-build.sh target/web
python3 -m http.server -d target/web 8000                  # then open http://localhost:8000
```

CI: `.github/workflows/pages.yml` builds it into `site/app/` and deploys the page. It also
pushes the build alone to the branch `web-preview` (one commit, replaced on each run), so it
can be fetched and served from a checkout.

## On an iPad

Open the page in Safari, touch the panel once to start the sound, then Share > Add to Home
Screen: the app opens full screen from its icon.
