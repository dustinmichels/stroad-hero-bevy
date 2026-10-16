# Web Build Plan: Stroad Hero in the Browser

Goal: ship Stroad Hero as a WebAssembly build that runs in any modern browser and deploys to a static host (itch.io / GitHub Pages).

Ground rule: **desktop stays the primary dev loop.** Nothing here may change `cargo run` / `mise run dev` behavior. Web-only settings live in `index.html`, the `wasm-release` profile, `[target.wasm32-unknown-unknown]` sections, or `#[cfg(target_arch = "wasm32")]` code.

Status (Bevy 0.20, verified 2026-10-15): with only the `index.html` below and the one-line canvas binding from §3, `trunk build --release --cargo-profile wasm-release` builds and links (about 9 min cold on this machine), and the game runs in Chromium with no page errors: 800×800 canvas, traffic running, arrow keys don't scroll the page. No other web-specific configuration was needed.

## 1. Toolchain setup

```bash
rustup target add wasm32-unknown-unknown
brew install trunk binaryen   # trunk = build + dev server; binaryen = wasm-opt
# or: cargo install --locked trunk
```

Trunk downloads a `wasm-bindgen-cli` matching `Cargo.lock` automatically; no manual install needed.

## 2. Project configuration

### `index.html` (repo root)

Trunk's entry point. Cargo ignores it, so it cannot affect desktop builds.

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Stroad Hero</title>
    <link data-trunk rel="rust" data-wasm-opt="z" />
    <style>
      html,
      body {
        margin: 0;
        height: 100%;
        background: #111;
        display: grid;
        place-items: center;
      }
      #game {
        outline: none; /* Bevy focuses the canvas; hide the browser focus ring */
      }
    </style>
  </head>
  <body>
    <canvas id="game"></canvas>
  </body>
</html>
```

Add `<link data-trunk rel="copy-dir" href="assets" />` once an `assets/` directory exists; Trunk fails on a missing directory.

### Bevy features: no change needed

Bevy's default `default_platform` set already includes `webgl2`, so the default feature set works on web. Do **not** copy feature lists from older Bevy guides: Cargo applies a dependency's features to every target, so a web-trimmed list also strips desktop (window backends `x11`/`wayland`, `multi_threaded`) and can drop things the game uses (`bevy_state` for `AppState`, `bevy_sprite_render` for `Mesh2d`/`Sprite` rendering).

If the size pass (§5) calls for trimming, change features for **both** targets at once, starting from the 0.20 collections:

```toml
bevy = { version = "0.20.0", default-features = false, features = ["2d", "ui", "audio"] }
```

`2d` = `default_app` (incl. `bevy_state`) + `default_platform` (incl. `webgl2`, `x11`, `wayland`, `multi_threaded`) + 2D rendering + gizmos. This rules out 3D (see README); add `3d` back if the traffic model goes 3D. Verify with `cargo run` and `trunk serve` after any change.

### getrandom: not needed

Bevy 0.20 compiles for `wasm32-unknown-unknown` without the `getrandom_backend` rustflag or a `getrandom` dependency. Only revisit if a crate that needs OS randomness (e.g. `rand`) is added; then follow that `getrandom` version's WebAssembly docs, keeping the config under `[target.wasm32-unknown-unknown]` / `[target.'cfg(target_arch = "wasm32")'.dependencies]`.

### Release profile (`Cargo.toml`)

`[profile.wasm-release]` already exists (`opt-level = "s"`, `strip = "debuginfo"`, inherits `codegen-units = 1`, `lto = "thin"` from `release`). It is used only for web builds, so these overrides are safe to try during the size pass; keep whichever wins:

```toml
opt-level = "z"
lto = "fat"
panic = "abort"
```

Build with: `trunk build --release --cargo-profile wasm-release`. Never pass `--features dynamic_linking` to a web build.

## 3. Code changes

- [ ] **Canvas binding**: add `canvas: Some("#game".into())` to the existing `Window` in `src/main.rs`. Native builds ignore this field, so no `cfg` gate is needed. Keep the fixed 800×800 `resolution` and leave `fit_canvas_to_parent` at its default (`false`): the world, spawn/despawn margins, and player clamping assume an 800×800 view at 1 world unit per pixel, and a canvas stretched to the browser would show past the playfield. `prevent_default_event_handling` already defaults to `true` (arrow keys won't scroll the page).

The items below are constraints for future features; the current code has none of these.

- [ ] **Asset loading**: assets are fetched over HTTP; load everything through `AssetServer`, never `std::fs`.
- [ ] **Asset meta files**: if the host 404s on `.meta` lookups, set `AssetPlugin { meta_check: AssetMetaCheck::Never, ..default() }`.
- [ ] **Audio unlock**: browsers block audio until the user interacts. Add a "Click / press any key to start" state before any sound plays.
- [ ] **No threads**: avoid `std::thread` and blocking calls; use Bevy systems/tasks.
- [ ] **Saves / high scores**: no `std::fs` on web; use `localStorage` (`web-sys`) behind `#[cfg(target_arch = "wasm32")]`, or a crate like `bevy_pkv`.
- [ ] **Desktop-only code**: gate with `#[cfg(not(target_arch = "wasm32"))]` (e.g. a "Quit" button, window-close handling).

## 4. Local testing

```bash
trunk serve            # http://127.0.0.1:8080, auto-rebuilds
```

- [ ] Test in Chrome, Firefox, Safari.
- [ ] Test keyboard controls don't scroll the page.
- [ ] Test on a phone (touch input is a separate task; see stretch goals).
- [ ] Watch the browser console for panics. Bevy's default `PanicHandlerPlugin` already installs `console_error_panic_hook` on wasm.
- [ ] Re-run `cargo run` to confirm desktop is unchanged.

## 5. Size budget

Target: **< 15 MB** wasm (uncompressed), ideally under 10 MB.

- [x] Record baseline size after first build: **29.4 MB** `.wasm` (default features, `wasm-release`, `data-wasm-opt="z"`). Over budget; the trims below are needed.
- [ ] Try the `wasm-release` overrides from §2 (`opt-level = "z"`, `lto = "fat"`, `panic = "abort"`).
- [ ] Trim Bevy features (see §2: both targets, verify both).
- [ ] Compress audio (OGG) and images (PNG via `oxipng`).
- [ ] Hosts that serve gzip/brotli cut transfer size roughly 3–4×.

## 6. Deployment

### GitHub Pages (CI)

`.github/workflows/web.yml`:

```yaml
name: Web build
on:
  push:
    branches: [main]
permissions:
  contents: read
  pages: write
  id-token: write
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - uses: jetli/trunk-action@v0.5.0
      - run: trunk build --release --cargo-profile wasm-release --public-url /${{ github.event.repository.name }}/
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist
  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment: github-pages
    steps:
      - uses: actions/deploy-pages@v4
```

- [ ] Enable Pages → Source: GitHub Actions in repo settings.

### itch.io

- [ ] `trunk build --release --cargo-profile wasm-release --public-url ./`
- [ ] Zip the **contents** of `dist/` (with `index.html` at the zip root) and upload as an HTML game; set the embed size to 800×800.
- [ ] Optional: automate with `butler push dist <user>/stroad-hero:web`.

## 7. Milestones

| #   | Milestone                | Done when                                       |
| --- | ------------------------ | ----------------------------------------------- |
| 1   | Toolchain + `index.html` | `trunk serve` shows the game; `cargo run` same  |
| 2   | Web compatibility fixes  | Plays start to finish with no console errors    |
| 3   | Size pass                | wasm under budget; desktop still builds and runs |
| 4   | CI deploy                | Push to `main` updates the public URL           |
| 5   | itch.io page             | Playable embed is live                          |

## Stretch goals

- Responsive canvas: `fit_canvas_to_parent: true` plus a camera projection that always shows the 800×800 world (`ScalingMode::AutoMin { min_width: 800.0, min_height: 800.0 }`), applied on both targets so desktop could also become resizable.
- Touch controls (on-screen d-pad or swipe-to-hop) for mobile play.
- Loading screen / progress bar in `index.html` while the wasm downloads.
- WebGPU build (`webgpu` feature) behind a separate target, falling back to WebGL2.
- Share-your-score link so players can spread the anti-stroad message.
