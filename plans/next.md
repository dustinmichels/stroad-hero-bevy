## Stretch Goals & Future Work (Post-Release)

- **Minimap**: render debug 2D view into a corner viewport while playing in 3D (requires gizmo `RenderLayers` rather than `ViewMode` run condition).
- **Responsive canvas**: configure `fit_canvas_to_parent: true` with camera projection preserving 800×800 aspect ratio across desktop and web.
- **Mobile touch controls**: on-screen virtual d-pad or swipe-to-hop controls.
- **Web loading screen**: CSS progress bar or animated loading indicator in `index.html` while `.wasm` downloads.
- **WebGPU target**: build target with `webgpu` feature falling back gracefully to WebGL2.
- **Camera rotation & remap**: allow camera yaw changes with automatic arrow-key remapping (`docs/isometric-view.md` §3.4).
- **Full pixel-art pipeline**: low-resolution render target (320×320), nearest-neighbor upscale, custom palette, and 3D pixel-art assets (`docs/isometric-view.md`).

## Deferred Validation & Distribution

These tasks need hardware, browsers, credentials, deployed infrastructure, or features that are not currently present:

- **Firefox and Safari compatibility**: run `env -u NO_COLOR trunk serve`; verify rendering, WASD/arrow-key `preventDefault`, view toggling, HUD stability, and a clean WebGL2 console.
- **Physical mobile validation**: verify layout and frame rate, including high traffic density, on representative mobile hardware.
- **Alternative car shape**: evaluate a long `Capsule3d` after box-car rendering is complete.
- **Runtime asset compression**: compress shipped audio with OGG and images with `oxipng` once runtime assets exist.
- **Runtime asset bundling**: add `<link data-trunk rel="copy-dir" href="assets" />` once the game has an `assets/` directory containing shipped `.glb`, texture, or audio files.
- **Asset metadata workaround**: configure `AssetPlugin { meta_check: AssetMetaCheck::Never, ..default() }` only if static hosting produces `.meta` 404s.
- **Browser audio unlock**: add a click-to-start interaction before browser audio is introduced.
- **Web persistence**: use `localStorage` (`web-sys`) or `bevy_pkv` when high scores or save data are introduced.
- **GitHub Pages release**: push the finished release candidate to `main` and verify the Pages workflow deploys a live playable site.
- **itch.io release**: upload the tested release zip and verify the 800×800 canvas iframe.
