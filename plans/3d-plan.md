# 3D View & Web Build Plan: 3D Main View + 2D Debug View + Wasm Deployment

Goal: render the game in 3D (player = pill, cars = boxes/capsules) as the main view, replace 2D meshes/sprites with a lightweight gizmo-based 2D debug view toggled via UI button or key, and ship the result as a WebAssembly build running smoothly on static hosting (GitHub Pages and itch.io).

This unified plan integrates the 3D graphics pipeline with the concurrent and post-3D web build requirements. This serves as the plain primitives foundation before the isometric pixel-art style in `docs/isometric-view.md`.

---

## Architectural Principles & Constraints

### 3D Design Decisions

- **Simulation stays 2D.** All gameplay mechanics operate on the ground plane ($X, Y$). Gameplay systems read and write `GroundPos(Vec2)` + `Heading(f32)`, never Bevy's `Transform`. Vertical elevation, if ever needed, becomes an independent `Elevation` component.
- **3D is the only art pipeline.** All future visual assets (buildings, vehicles, signs, splats) are authored exclusively for 3D.
- **The 2D view is debug lines, not art.** Gizmos draw the player circle, car boxes, road outlines, and splat markers directly from simulation state each frame. It requires zero maintenance when 3D art changes.
- **Coordinate mapping.** 2D ground `(x, y)` maps to 3D `(x, h, -y)`. Heading $\theta$ (radians, CCW from +X) maps to `Quat::from_rotation_y(θ)`. This keeps north away from the camera and preserves right-hand traffic rules.
- **Entity linking.** 3D visual representations are discrete entities tied to simulation entities by relationships (`View3dOf(Entity)`, `linked_spawn`). Despawning a car despawns its visual 3D mesh automatically.
- **Toggle = camera switch.** A `ViewMode { ThreeD, TopDown }` state (default `ThreeD`) flips `Camera.is_active`. Gizmo drawing systems only execute in `TopDown`, preventing debug lines from rendering in 3D.
- **Fixed camera orientation.** The 3D camera faces north (yaw 0) tilted downward, maintaining existing arrow/WASD ground mapping (+X = right, +Y = up/north) without input remapping.
- **Scale parity.** 1 world unit = 1 pixel of the previous 2D view. 3D heights are constants defined in the same unit space.

### Web Ground Rules & Platform Limits

- **Desktop stays the primary dev loop.** Nothing here alters native `cargo run` / `mise run dev` workflows. Web-only configurations live in `index.html`, the `wasm-release` profile, `[target.wasm32-unknown-unknown]`, or `#[cfg(target_arch = "wasm32")]` blocks. Never pass `--features dynamic_linking` to web builds.
- **WebGL2 limits.** Browsers lacking WebGPU fall back to WebGL2. Keep materials simple (`StandardMaterial` with flat colors or a single shared palette texture; unlit for decals). Keep lighting to 1 `DirectionalLight` and 1 `AmbientLight`. Use a single shadow cascade without multi-cascade shadow maps.
- **HTTP asset loading.** Load all assets through `AssetServer`, never `std::fs` (web fetches assets over HTTP).
- **No blocking threads.** Do not use `std::thread` or blocking calls; use Bevy systems and async compute tasks.
- **Desktop-only code isolation.** Gate native-only features (e.g. window exit handlers) with `#[cfg(not(target_arch = "wasm32"))]`.
- **Shared file hygiene.** Avoid touching `[profile.wasm-release]` or Bevy default features in `Cargo.toml` until the 3D size checkpoint is recorded, ensuring measurements isolate 3D overhead.

---

## Status (Bevy 0.20, Updated 2026-10-15)

- **Simulation Decoupling (Phase 1)**: Done. `GroundPos`/`Heading` implemented in `src/sim.rs`, movement/collision/car-cleanup/splats ported, 69 unit tests passing.
- **3D View Foundations (Phase 5.1)**: Underway. `src/view3d.rs` created with `View3dPlugin`, coordinate conversion helpers and unit tests added, `Camera3d` perspective camera spawned.
- **Baseline Web Build**: Verified. `index.html` at root, canvas bound to `#game`, runs in Chromium at 800×800 with no page errors, traffic running, and arrow keys prevented from scrolling page. Baseline uncompressed wasm size: **29.4 MB** (Bevy default features).
- **Traffic Behavior & Stoplight (Phase 6)**: Planned. Add route-aware following, finite acceleration/braking and reaction delay, rear-end crashes, and one shared red/yellow/green intersection signal after the 3D primitives are playable.

---

## Master Todo List

### Phase 1: Simulation Decoupling (Prerequisite) [DONE]

- [x] Add `GroundPos(pub Vec2)` and `Heading(pub f32)` components in `src/sim.rs`.
- [x] Spawn the player and cars with `GroundPos` + `Heading` (`setup_player`, `spawn_car_in_lane`).
- [x] Port `player_movement` (and `clamp_position`) to `GroundPos`.
- [x] Port `car_movement` (straight and curve branches) to write `GroundPos`/`Heading` instead of `transform.translation` and `Quat::from_rotation_z`.
- [x] Port `car_cleanup` to `GroundPos`.
- [x] Port `collision_detection` to `GroundPos`/`Heading`, removing the `2.0 * rotation.z.atan2(rotation.w)` angle recovery.
- [x] Port `restart_game` and post-collision player reset to write `GroundPos`.
- [x] Normalize car footprint: `Car` stores `length`/`width` once, and `Heading` handles rotation (removes per-lane dimension swapping in `TrafficLane::car_dimensions`).
- [x] Give `BloodSplat` `GroundPos` and seed for view-agnostic rendering.
- [x] Update existing unit tests to set and assert `GroundPos`/`Heading` instead of `Transform`.
- [x] Add temporary bridge `sync_2d` system writing `Transform` from `GroundPos`/`Heading` for legacy meshes (deleted in Phase 8).
- [x] Verify `cargo test` is green (69 passed) and `cargo run` plays identically to before.

---

### Phase 2: Web Toolchain & Project Configuration [DONE]

- [x] Install wasm compilation target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- [x] Install Trunk and Binaryen:
  ```bash
  brew install trunk binaryen   # trunk = build + dev server; binaryen = wasm-opt
  # or: cargo install --locked trunk
  ```
  _(Note: Trunk automatically downloads matching `wasm-bindgen-cli` based on `Cargo.lock`.)_
- [x] Create Trunk entry point `index.html` at repository root:
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
- [x] Configure canvas binding on `Window` in `src/main.rs`:
  - `canvas: Some("#game".into())`
  - Fixed 800×800 `resolution`
  - `fit_canvas_to_parent: false`
  - `prevent_default_event_handling: true`
- [x] Verify Bevy 0.20 compiles without `getrandom` backend overrides (OS randomness is unused; only configure if `rand` is added).
- [x] Verify baseline web compilation:
  ```bash
  trunk build --release --cargo-profile wasm-release
  ```
  _(Baseline uncompressed wasm size: 29.4 MB)._

---

### Phases 3–4: Web Preflight & Deployment (Parallel / Now)

Tackle the preflight and deployment workstreams in parallel with each other and with the 3D graphics work; neither workstream changes simulation or rendering code.

#### Workstream A: Browser Preflight & Asset Directory Cleanup

- [x] Move source asset `assets/car.blend` (~546 KB) to `art/car.blend` to keep unneeded build assets out of web distribution bundles.
- [x] Delete `assets/.DS_Store` and ensure `.DS_Store` is present in `.gitignore`.
- [x] Verify browser developer console is clean with no WebGL/JS panics.
- [x] Verify desktop build (`cargo run` / `mise run dev`) remains unaffected.

_(Handoff notes: Baseline 2D web build verified on `env -u NO_COLOR trunk serve` at port 8080. Evaluated in Chromium: WebGL 2.0 ANGLE Metal backend initialized, 0 errors, 0 panics, keyboard event preventDefault active on WASD/arrows. `Trunk.toml` added scoping watch to `["src", "index.html"]`. Desktop build passes all 73 tests and launches via Metal renderer.)_

#### Workstream B: Deployment Pipeline Setup

- [x] Add GitHub Pages deployment workflow at `.github/workflows/web.yml`:
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
- [x] Enable GitHub Pages under Repository Settings → Pages → Source: GitHub Actions.
- [x] Document and verify itch.io release packaging:
  - Command: `trunk build --release --cargo-profile wasm-release --public-url ./`
  - Zip contents of `dist/` (`index.html` at zip root), upload as HTML game with 800×800 embed dimensions.
  - (Optional) Automate itch upload: `butler push dist <user>/stroad-hero:web`.

_(Handoff notes: GitHub Pages workflow added at `.github/workflows/web.yml` and verified via Ruby YAML parse. GitHub Pages enabled on repository `dustinmichels/stroad-hero` with `build_type: workflow`. Release build verified via `trunk build --release --cargo-profile wasm-release --public-url ./`, `dist/index.html` confirmed using relative `./` paths, and `stroad-hero-web.zip` verified having `index.html` at root via `unzip -l`. Packaging steps documented in `README.md`.)_

---

### Phase 5: 3D View Core Implementation

#### 5.1 3D Foundations

- [x] Create `src/view3d.rs` with `View3dPlugin`, registered in `build_app`.
- [x] Add constants: `PLAYER_HEIGHT_3D`, `CAR_HEIGHT_3D`, `CAMERA3D_PITCH`, `CAMERA3D_DISTANCE`, `CAMERA3D_FOV`.
- [x] Add pure helpers `ground_to_3d(pos: Vec2, h: f32) -> Vec3` and `heading_to_3d(θ: f32) -> Quat`.
- [x] Unit-test helpers: +X stays +X, +Y becomes -Z, and heading $\pi/2$ points car long axis toward -Z.
- [x] Spawn perspective `Camera3d` tagged with `ThreeDCamera`.
      _(Note: Camera setup is registered in `View3dPlugin` in `src/view3d.rs` with perspective projection and clamp logic.)_
- [ ] Add `DirectionalLight` (single shadow cascade for WebGL2 compliance) and `AmbientLight` so box sides read clearly.
      _(Next up: In `src/view3d.rs`, add directional light with single cascade shadows and ambient light in `setup_camera_3d` or a dedicated setup system in `View3dPlugin`.)_
- [ ] Add `camera3d_follow_player`: player ground point plus pitch/distance offset, `looking_at` player, clamped matching 2D camera bounds.
- [ ] Add ground plane (`Plane3d`, `WORLD_WIDTH × WORLD_HEIGHT`, grass color) sized to prevent seeing past the world boundary at pitch angle.

#### 5.2 World & Entity Rendering (Parallel Workstreams)

After the shared 3D resources exist, tackle roads, the player, cars, and blood splats in parallel. Coordinate ownership of shared mesh/material handles first so the workstreams do not introduce per-entity allocations or competing resource types.

##### Roads as Shared Data, Drawn in 3D

- [ ] Add pure function `road_pieces(&RoadGeometry) -> Vec<RoadPiece>`, where `RoadPiece { center: Vec2, size: Vec2, angle: f32, kind: Asphalt | Centerline | Border }`.
- [ ] Refactor `setup_roads` to spawn `Road`/`Lane` data entities without `Sprite`.
- [ ] Spawn thin flat meshes from `road_pieces` in `View3dPlugin`: asphalt at $y = 0.0$, markings at $y = 0.1$ to avoid z-fighting. Share mesh handles per shape and material handles per kind.
- [ ] Visual check: intersection and north-bending curve match original 2D layout.
- [ ] Verify road tests pass (road entity count = 3, geometry queries).

##### Player Pill

- [ ] Add `On<Add, Player>` observer spawning `Capsule3d` (radius `PLAYER_RADIUS`, height `PLAYER_HEIGHT_3D`) with `StandardMaterial` in `COLOR_PLAYER`, plus `View3dOf(player)`.
- [ ] Implement `sync_3d_player`: update translation to `ground_to_3d(pos, half_height)` so pill stands flush on the ground plane.
- [ ] Update player material color dynamically based on `Player.alive` (`COLOR_PLAYER` / `COLOR_PLAYER_DEAD`), replacing 2D `ColorMaterial` modifications.

##### Car Boxes & Curve Navigation

- [ ] Add `On<Add, Car>` observer spawning `Cuboid` (`length × CAR_HEIGHT_3D × width`, long axis = local X) plus `View3dOf(car)`.
- [ ] Allocate shared mesh handles and small palette of shared materials (zero per-car allocation).
- [ ] Implement `sync_3d_cars`: sync translation from `GroundPos` and rotation from `heading_to_3d(Heading)`.
- [ ] Visual check: cars traverse straight lanes and curves with smooth orientation transitions.
- [ ] Verify car despawning and game restart remove associated 3D boxes via `linked_spawn`.

##### Blood Splats

- [ ] Add `On<Add, BloodSplat>` observer spawning decal from `create_blood_splat_mesh(seed, …)`, rotated $-\pi/2$ about X to lie flat at $y = 0.05$, using unlit `StandardMaterial`.
- [ ] Verify game restart cleans up splat decals.

#### 5.3 Traffic-Feature Rendering Prerequisites

- [ ] Keep traffic decisions in simulation data (`GroundPos`, `Heading`, lane/progress, speed, signal state); views only render that state.
- [ ] Expose shared car mesh/material handles so normal, braking, and crashed cars can change appearance without allocating materials per car.
- [ ] Keep stop-line and signal placement derived from `RoadGeometry`, not duplicated world-coordinate constants.

---

### Phase 6: Driving Logic, Queues, Rear-End Crashes & Stoplight

Scope decision: implement one intersection-wide signal shared by all four approaches. It cycles `Green → Yellow → Red → Green`; coordinated horizontal/vertical right-of-way phases, turning arrows, and cross-lane intersection crashes are intentionally deferred. Cars already past their stop line always clear the intersection.

#### 6.1 Traffic Foundations (Parallel Workstreams)

Tackle route progress, the signal state machine, and finite acceleration in parallel. Agree on the `CarMotion` fields and progress/stop-line helper signatures first; queue planning in 6.2 is the integration point for all three workstreams.

##### Route Progress: One Coordinate for Straight and Curved Lanes

- [ ] Add pure `lane_progress(lane, position, &RoadGeometry) -> f32`, measured from that lane's spawn end and increasing in the travel direction.
- [ ] Cover all route segments:
  - Eastbound: west straight → quarter-circle curve → north straight.
  - Westbound: southbound straight from the north edge → reverse quarter-circle → west straight.
  - Northbound and Southbound: direct vertical progress.
- [ ] Add `stop_line_progress(lane, &RoadGeometry) -> f32` for a car center stopped one half-car-length plus a small buffer before `intersection_bounds`.
- [ ] Add `front_progress`/`rear_progress` helpers using `Car.length`; following and collision code must compare bumper gaps, not center distances.
- [ ] Unit-test monotonic progress immediately before/after each straight/curve boundary, both curve directions, and all four stop lines.

##### Signal State Machine

- [ ] Add `TrafficLightState { Green, Yellow, Red }` and a `TrafficSignal` resource holding the current state and repeating phase timer.
- [ ] Add named initial tuning constants: green `6.0s`, yellow `1.5s`, red `5.0s`. Keep these compile-time constants until playtesting proves runtime configuration is needed.
- [ ] Implement `tick_traffic_signal` with deterministic transitions `Green → Yellow → Red → Green`; initialize at `Green`.
- [ ] Define approach behavior:
  - Green: no signal speed restriction.
  - Yellow: stop if the car can do so at comfortable deceleration before the line; otherwise commit to crossing.
  - Red: stop before the line unless the car committed during yellow or its front bumper already crossed the line.
- [ ] Store the yellow commitment per car and clear it after the rear bumper passes the stop line so a red transition cannot strand a car inside the intersection.
- [ ] Unit-test exact phase transitions, timer rollover, yellow stop/go decisions on both sides of the stopping-distance boundary, and the red-after-yellow commitment case.

##### Car Motion State and Finite Acceleration

- [ ] Separate design speed from instantaneous motion: keep `CAR_SPEED` as cruise speed and add per-car `CarMotion { current_speed, desired_speed, perceived_speed_limit, pending_speed_limit, reaction_remaining, committed_to_cross }`.
- [ ] Initialize spawned cars at cruise speed; update legacy tests and direct test fixtures to include `CarMotion`.
- [ ] Add initial tuning constants:
  - `CAR_ACCELERATION = 160.0` units/s².
  - `CAR_COMFORTABLE_DECELERATION = 300.0` units/s².
  - `CAR_EMERGENCY_DECELERATION = 520.0` units/s².
  - `DRIVER_REACTION_SECS = 0.35`.
  - `CAR_MIN_FOLLOW_GAP = 14.0` units.
- [ ] Add a pure speed-step helper that clamps acceleration/deceleration by `dt`, never produces a negative speed, and never overshoots the desired speed.
- [ ] Change `car_movement` to advance route position by `current_speed * dt`; retain its existing lane snapping, curve math, headings, and cleanup behavior.
- [ ] Unit-test acceleration from rest, comfortable braking, emergency braking, zero-speed stability, and a single large frame that must not move farther than the computed speed step permits.

#### 6.2 Leader Detection, Driver Reaction, and Queue Formation

- [ ] Add `plan_car_motion` before `car_movement`. Group live cars by `TrafficLane`, order each group by `lane_progress`, and compute each follower's bumper gap to the nearest car ahead.
- [ ] Reuse scratch vectors through Bevy `Local` state; do not allocate fresh lane lists every frame.
- [ ] Compute each car's desired speed as the most restrictive of:
  - Cruise speed.
  - The speed that preserves `CAR_MIN_FOLLOW_GAP` behind the leader.
  - The speed that stops at a red/yellow stop line.
- [ ] Use $v_\text{safe} = \sqrt{2ad}$ with non-negative available distance `d` and the appropriate deceleration constant; clamp invalid/overlapping gaps to zero.
- [ ] Model reaction delay only when a newly observed limit is stricter: hold the old perceived limit while `reaction_remaining > 0`, retain the strictest pending limit without restarting the timer, then apply it. Relaxed limits may apply immediately so queues start moving promptly on green.
- [ ] Apply emergency deceleration when the reacted safe speed is below what comfortable braking can reach this frame; emergency braking is finite, so an abrupt stop can still produce a collision.
- [ ] Unit-test a three-car queue: lead car brakes for red, followers react in order, all stop without overlap at adequate spacing, and all accelerate after green.
- [ ] Unit-test an abrupt stopped leader at two gaps: adequate distance avoids contact; insufficient distance creates overlap despite emergency braking.

#### 6.3 Rear-End Collision and Crash Lifecycle

- [ ] Add `detect_rear_end_crashes` after movement. Compare adjacent cars in the same lane using route progress and bumper gap; a gap `<= 0` is a rear-end collision.
- [ ] Add a `Crashed` component with a cleanup timer. On impact, mark both cars crashed and set all motion speeds/limits to zero; crashed cars remain physical obstacles until cleanup.
- [ ] Ignore already-crashed pairs when emitting new impacts so one overlap does not refresh timers or duplicate effects every frame.
- [ ] Despawn crashed cars after a short named duration (start at `3.0s`) to prevent permanent road blockage and entity leakage.
- [ ] Prevent `spawn_cars` from spawning into an occupied spawn footprint; defer that lane's spawn until enough bumper clearance exists rather than creating an immediate crash off-screen.
- [ ] Keep player/car collision behavior unchanged; this slice handles only same-lane rear-end crashes.
- [ ] Add tests for one impact transition, both cars stopping, no duplicate impact on the next frame, timed cleanup, adjacent-lane non-collision, and blocked-spawn deferral.

#### 6.4 Stoplight and Driving-State Visuals

Once the `TrafficSignal`, `CarMotion`, and `Crashed` contracts are fixed, tackle the 3D signal heads, shared road markings, and driving-state palette in parallel with 6.2–6.3. Add the top-down debug pieces when the Phase 8 debug view exists.

- [ ] Add four approach-facing signal heads driven by the one `TrafficSignal` resource so the shared aspect is visible from every lane.
- [ ] Build each 3D head from shared primitive meshes/materials: pole/housing plus red, yellow, and green lenses. Change visibility/emissive state on transitions without creating new materials.
- [ ] Add one stop-line marking per approach through shared road-piece data; render it in both 3D and the top-down debug view.
- [ ] Show braking and crash state with shared palette materials (for example, brake-light red and a distinct crashed-car color); do not put simulation decisions in rendering systems.
- [ ] Extend `draw_debug_2d` with compact signal-state markers and crashed-car outlines so behavior can be diagnosed without 3D meshes.
- [ ] Verify signal visuals, stop lines, brake state, and crash state in both camera modes and in the web build.

#### 6.5 Scheduling, Integrated Scenarios, and Done Criteria

- [ ] Order traffic systems explicitly: `tick_traffic_signal → spawn_cars → plan_car_motion → apply_car_acceleration → car_movement → detect_rear_end_crashes → tick_crash_cleanup → car_cleanup`.
- [ ] Keep the chain gated by `AppState::InGame`; verify non-game states freeze the signal phase, driver reaction, movement, and crash cleanup timers.
- [ ] Update restart handling to reset the signal to `Green`, clear queued/crashed cars through the existing entity cleanup path, and reset spawn timing.
- [ ] Add an integrated deterministic scenario covering a complete cycle: cars enter on green, queue on red without crossing the line, restart on green, and one deliberately short-gap follower crashes after an abrupt stop.
- [ ] Native smoke: watch at least two full signal cycles, verify every approach obeys the same aspect, queues do not overlap at normal spacing, yellow-committed cars clear the intersection, crashes remain possible at insufficient spacing, wrecks clear, and spawning recovers.
- [ ] Browser smoke: run the same scenario under `trunk serve`; verify stable motion through a throttled/long frame and no console panic.

---

### Phase 7: Wasm Size Checkpoint & Profile Optimization

Target: **< 15 MB** uncompressed wasm (ideally < 10 MB). Baseline before 3D: 29.4 MB.

After recording the checkpoint, tackle profile tuning and transfer-compression verification in parallel; compare each result against the same checkpoint build.

- [ ] **3D Size Checkpoint**: build wasm after `View3dPlugin` primitive rendering is complete:
  ```bash
  trunk build --release --cargo-profile wasm-release
  ```
  Record the uncompressed `.wasm` size. If size blows the budget significantly, assess before continuing.
- [ ] **Profile overrides**: test adjustments in `[profile.wasm-release]` in `Cargo.toml` and adopt wins:
  ```toml
  opt-level = "z"
  lto = "fat"
  panic = "abort"
  ```
- [ ] **Verify transfer compression**: confirm static server gzip / brotli compression reduces transfer footprint ~3–4× (e.g. 29 MB raw $\to$ ~7–9 MB over wire).

---

### Phase 8: Replace 2D Meshes with Gizmo Debug View & Feature Trimming

- [ ] Create `src/view_debug2d.rs` with `DebugView2dPlugin`. Move `Camera2d` spawn and follow logic (`setup_camera`, `camera_follow_player`) from `main.rs`, tagging the camera `TopDownCamera`.
- [ ] Implement `draw_debug_2d` gizmos: player circle (colored by `alive`), car boxes as rotated rects from `GroundPos`/`Heading`/footprint, road outlines from `road_pieces`, and an X marker for each blood splat.
- [ ] Delete obsolete 2D visual code: `Mesh2d`/`MeshMaterial2d` on player, cars, and splats; road `Sprite` components; `Z_*` layering constants; and the temporary `sync_2d` bridge from Phase 1.
- [ ] Update existing tests that query `Mesh2d` / `ColorMaterial` (`test_setup_player_spawns_entity`, collision color tests).
- [ ] **Feature trimming evaluation**: Bevy defaults are `["2d", "3d", "ui", "audio"]`. In Bevy 0.20, `LineGizmo2dPlugin` on `Camera2d` depends on `bevy_sprite_render` (enabled only by `2d`). Choose one:
  - Option A: Keep `2d` feature and trim elsewhere.
  - Option B: Render 2D debug view using an orthographic top-down `Camera3d` with 3D gizmos, allowing:
    ```toml
    bevy = { version = "0.20.0", default-features = false, features = [
      "3d",
      "ui",
      "audio",
      "default_platform",
      "default_app",
    ] }
    ```
    _(Note: Never omit `default_platform` or `default_app`.)_
- [ ] Test both `cargo run` and `trunk serve` after any feature adjustments.

---

### Phase 9: View Toggle (3D Main View / 2D Debug View)

- [ ] Add `ViewMode { ThreeD, TopDown }` as a Bevy `States` (default `ThreeD`).
- [ ] Add `OnEnter(ViewMode::*)` systems flipping `is_active` on `ThreeDCamera` and `TopDownCamera`.
- [ ] Restrict `draw_debug_2d` system execution to run only in `ViewMode::TopDown`.
- [ ] Add top-left UI `Button` displaying the alternative view mode, toggling `ViewMode` on `Interaction::Pressed`.
- [ ] Add keyboard shortcut `V` for the view toggle (avoids conflict with WASD/arrows, `R`, `Space`).
- [ ] Ensure Bevy UI / HUD (SPD counter, toggle button) remains active and properly attached across view switches (manage `IsDefaultUiCamera` or `UiTargetCamera`).

---

### Phase 10: Automated Verification & Testing

Once Phases 8–9 land, tackle the linking, toggle, simulation-independence, and integrated smoke tests in parallel, then run the complete suite once.

- [ ] **Linking test**: spawning a car produces exactly one `View3d` proxy whose `Transform` matches `ground_to_3d(GroundPos)`; despawning car removes proxy.
- [ ] **Toggle test**: button or `V` key flips `ViewMode`, exactly one camera is active, and toggling twice restores `ThreeD`.
- [ ] **Simulation independence test**: vehicle collisions and SPD increment logic behave identically in both `ViewMode::ThreeD` and `ViewMode::TopDown`.
- [ ] Update `test_full_integrated_app_smoke` headless test to include both view plugins (with `Assets<StandardMaterial>` mock setup).

---

### Phase 11: 3D Web Validation

Validate the release candidate in the locally available browser.

#### 11.1 Browser Validation

- [ ] **Browser compatibility**: test the 3D build in Chromium using `env -u NO_COLOR trunk serve`. Verify zero WebGL2 shader compilation failures or buffer errors in console.
- [ ] **Context & toggle stability**: verify switching views (`V` or button) occurs smoothly on web without frame drops or WebGL context losses.
- [ ] **HUD stability**: ensure SPD text and toggle button remain visible across both view modes in web canvas.
- [ ] **Depth test check**: verify gizmo debug lines render without depth-test clipping against clear color on web.

---

### Phase 12: Release Readiness & Documentation

After the native smoke passes, update release documentation.

- [ ] **Native smoke run**: execute `cargo run`, play in 3D, switch to debug 2D mid-traffic, confirm boxes/circles match positions, toggle back, crash, restart.
- [ ] Update control instructions in `README.md` (WASD/arrows, `R`, `V` / button).
- [ ] Check off Phase 1 decoupling and Phase 7 web distribution tasks in `docs/masterplan.md`.
- [ ] Update `docs/isometric-view.md` §5 (record plain-3D + gizmo debug implementation and recorded wasm sizes).

---

## Milestones Summary

| #     | Milestone                     | Target Deliverable                              | Done When                                                        |
| ----- | ----------------------------- | ----------------------------------------------- | ---------------------------------------------------------------- |
| **1** | **Simulation Decoupling**     | Pure 2D sim layer (`GroundPos`, `Heading`)      | `cargo test` passes 69 tests; 2D gameplay identical [DONE]       |
| **2** | **Toolchain & Baseline Web**  | Trunk build + canvas integration                | `trunk serve` runs 2D web build without errors [DONE]            |
| **3** | **Preflight & CI Pipeline**   | Clean assets + GitHub Pages CI                  | Pages workflow runs on push; itch packaging documented           |
| **4** | **3D Main View**              | Pill player, box cars, flat roads, lights       | 3D gameplay runs with smooth curve navigation                    |
| **5** | **Traffic Behavior**          | Queues, finite braking, rear-end crashes, light | Cars stop/go by signal and react safely unless distance is short |
| **6** | **Gizmo Debug View & Toggle** | `ViewMode` switch between 3D & 2D gizmos        | UI button / `V` flips camera cleanly                             |
| **7** | **Wasm Size & Compatibility** | Size pass + Chromium QA                         | Uncompressed wasm < 15 MB; local web smoke passes                |
| **8** | **Release Candidate**         | Tested web release package                      | Native and Chromium smoke checks pass; release docs are current  |

---

## Re-org

- [ ] **Bevy best-practices review**: Audit the repository against current Bevy project best practices, with particular attention to thoughtful ECS design (clear component/resource boundaries, focused systems, explicit scheduling, event/observer use, query efficiency, change detection, and avoiding unnecessary entity/component churn); refactor confirmed issues and verify native and web behavior remains unchanged.
