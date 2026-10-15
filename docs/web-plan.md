# Web Build Plan: Stroad Hero in the Browser

Goal: ship Stroad Hero as a WebAssembly build that runs in any modern browser and deploys to a static host (itch.io / GitHub Pages).

## 1. Toolchain setup

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk          # build + dev server
cargo install --locked wasm-bindgen-cli # only if building manually; version must match Cargo.lock
brew install binaryen                  # provides wasm-opt for size reduction
```

- [ ] Confirm `trunk serve` works on a hello-world before touching game code.

## 2. Project configuration

### `index.html` (repo root)

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Stroad Hero</title>
    <link data-trunk rel="rust" data-wasm-opt="z" />
    <link data-trunk rel="copy-dir" href="assets" />
    <style>
      html,
      body {
        margin: 0;
        height: 100%;
        background: #111;
      }
      #game {
        width: 100%;
        height: 100%;
        display: block;
      }
    </style>
  </head>
  <body>
    <canvas id="game"></canvas>
  </body>
</html>
```

### `.cargo/config.toml`

Bevy pulls in `getrandom`, which needs an explicit web backend on wasm:

```toml
[target.wasm32-unknown-unknown]
rustflags = ['--cfg', 'getrandom_backend="wasm_js"']
```

And in `Cargo.toml`:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.3", features = ["wasm_js"] }
```

### Release profile for small builds (`Cargo.toml`)

`[profile.wasm-release]` already exists in `Cargo.toml` (the Bevy guide's version:
`opt-level = "s"`, `strip = "debuginfo"`, inheriting `codegen-units = 1` and
`lto = "thin"` from `release`). When measuring wasm size, try these overrides and keep
whichever wins:

```toml
opt-level = "z"
lto = "fat"
panic = "abort"
```

Build with: `trunk build --release --cargo-profile wasm-release`

### Trim Bevy features

Disable default features and opt in to only what a 2D game needs, e.g.:

```toml
bevy = { version = "<current>", default-features = false, features = [
  "bevy_asset",
  "bevy_audio",
  "bevy_core_pipeline",
  "bevy_render",
  "bevy_sprite",
  "bevy_text",
  "bevy_ui",
  "bevy_winit",
  "bevy_window",
  "default_font",
  "png",
  "vorbis",
  "webgl2",
] }
```

- [ ] Check the feature list against the Bevy version in use (names shift between releases).

## 3. Code changes

- [ ] **Canvas binding**: point the primary window at the HTML canvas.

  ```rust
  WindowPlugin {
      primary_window: Some(Window {
          canvas: Some("#game".into()),
          fit_canvas_to_parent: true,
          prevent_default_event_handling: true, // stops arrow keys scrolling the page
          ..default()
      }),
      ..default()
  }
  ```

- [ ] **Asset loading**: assets are fetched over HTTP. Remove any synchronous `std::fs` reads; everything goes through `AssetServer`.
- [ ] **Asset meta files**: if the host 404s on `.meta` lookups, set `AssetPlugin { meta_check: AssetMetaCheck::Never, ..default() }`.
- [ ] **Audio unlock**: browsers block audio until the user interacts. Add a "Click / press any key to start" title state before any sound plays.
- [ ] **No threads**: avoid `std::thread` and blocking calls; use Bevy systems/tasks.
- [ ] **Saves / high scores**: `std::fs` doesn't exist on web; use `web-sys` `localStorage` behind `#[cfg(target_arch = "wasm32")]`, or a crate like `bevy_pkv`.
- [ ] **Desktop-only code**: gate with `#[cfg(not(target_arch = "wasm32"))]` (e.g. "Quit" button, window-close handling).

## 4. Local testing

```bash
trunk serve            # http://127.0.0.1:8080, auto-rebuilds
```

- [ ] Test in Chrome, Firefox, Safari.
- [ ] Test keyboard controls don't scroll the page.
- [ ] Test on a phone (touch input is a separate task; see stretch goals).
- [ ] Watch the browser console for panics. Add `console_error_panic_hook` for readable stack traces in dev builds.

## 5. Size budget

Target: **< 15 MB** wasm (uncompressed), ideally under 10 MB.

- [ ] Record baseline size after first build.
- [ ] Apply release profile + `wasm-opt -Oz` (Trunk's `data-wasm-opt="z"` handles this).
- [ ] Trim Bevy features.
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
- [ ] Zip the **contents** of `dist/` (with `index.html` at the zip root) and upload as an HTML game.
- [ ] Optional: automate with `butler push dist <user>/stroad-hero:web`.

## 7. Milestones

| #   | Milestone                | Done when                                    |
| --- | ------------------------ | -------------------------------------------- |
| 1   | Toolchain + `index.html` | `trunk serve` shows the game                 |
| 2   | Web compatibility fixes  | Plays start to finish with no console errors |
| 3   | Size pass                | wasm under budget                            |
| 4   | CI deploy                | Push to `main` updates the public URL        |
| 5   | itch.io page             | Playable embed is live                       |

## Stretch goals

- Touch controls (on-screen d-pad or swipe-to-hop) for mobile play.
- Loading screen / progress bar in `index.html` while the wasm downloads.
- WebGPU build (`webgpu` feature) behind a separate target, falling back to WebGL2.
- Share-your-score link so players can spread the anti-stroad message.
