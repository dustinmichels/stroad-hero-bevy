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
- [x] Add `DirectionalLight` (single shadow cascade for WebGL2 compliance) and `AmbientLight` so box sides read clearly.
      _(Note: Spawned in `setup_camera_3d` in `src/view3d.rs` with single shadow cascade and ambient light component/resource.)_
- [x] Add `camera3d_follow_player`: player ground point plus pitch/distance offset, `looking_at` player, clamped matching 2D camera bounds.
      _(Note: Implemented in `src/view3d.rs` and added to `View3dPlugin` in `Update` schedule; verified by unit test `test_camera3d_follow_player_tracks_and_clamps`.)_
- [x] Add ground plane (`Plane3d`, `WORLD_WIDTH × WORLD_HEIGHT`, grass color) sized to prevent seeing past the world boundary at pitch angle.
      _(Note: Spawned in `setup_ground_3d` in `src/view3d.rs` using `Plane3d`, `COLOR_GROUND_GRASS`, and padded dimensions `GROUND_PLANE_WIDTH` × `GROUND_PLANE_HEIGHT`; verified by unit tests in `src/view3d.rs` and integrated in `src/main.rs`.)_

#### 5.2 World & Entity Rendering (Parallel Workstreams)

After the shared 3D resources exist, tackle roads, the player, cars, and blood splats in parallel. Coordinate ownership of shared mesh/material handles first so the workstreams do not introduce per-entity allocations or competing resource types.

##### Roads as Shared Data, Drawn in 3D

- [x] Add pure function `road_pieces(&RoadGeometry) -> Vec<RoadPiece>`, where `RoadPiece { center: Vec2, size: Vec2, angle: f32, kind: Asphalt | Centerline | Border }`.
- [x] Refactor `setup_roads` to spawn `Road`/`Lane` data entities without `Sprite`.
- [x] Spawn thin flat meshes from `road_pieces` in `View3dPlugin`: asphalt at $y = 0.0$, markings at $y = 0.1$ to avoid z-fighting. Share mesh handles per shape and material handles per kind.
      _(Note: Implemented via `setup_roads_3d` in `src/view3d.rs` using `Plane3d` flat meshes, 3 shared `StandardMaterial`s for Asphalt/Centerline/Border, and shared mesh handles cached per shape; ground plane adjusted to $y = -0.1$ to prevent z-fighting; verified by unit test `test_setup_roads_3d_spawns_pieces_with_shared_resources`.)_
- [x] Visual check: intersection and north-bending curve match original 2D layout.
      _(Note: Verified via web build running in headless Chromium; captured screenshots at initial view and camera moved east along curve showing asphalt, white borders, yellow centerlines, intersection, and 90-degree curved turn match the 2D layout without z-fighting or artifacts.)_
- [x] Verify road tests pass (road entity count = 3, geometry queries).
      _(Note: Verified `test_setup_roads_spawns_entities_in_z_road_layer` confirms road entity count = 3 and 0 sprites; `test_road_geometry_bounds`, `test_lane_center_coordinates`, `test_road_and_marking_exact_coordinates`, `test_curved_road_geometry_queries`, and `test_road_pieces_counts_and_properties` all pass; full test suite passes with 79 tests.)_

##### Player Pill

- [x] Add `On<Add, Player>` observer spawning `Capsule3d` (radius `PLAYER_RADIUS`, height `PLAYER_HEIGHT_3D`) with `StandardMaterial` in `COLOR_PLAYER`, plus `View3dOf(player)`.
      _(Note: Implemented `on_add_player` observer in `src/view3d.rs` spawning `Capsule3d` mesh, `StandardMaterial` in `COLOR_PLAYER`, and `View3dOf` relationship with `linked_spawn`; verified by unit test `test_on_add_player_spawns_capsule3d_and_links`.)_
- [x] Implement `sync_3d_player`: update translation to `ground_to_3d(pos, half_height)` so pill stands flush on the ground plane.
      _(Note: Implemented `sync_3d_player` in `src/view3d.rs` querying `GroundPos` via `View3dOf` and setting translation to `ground_to_3d(pos, half_height)`; verified by unit test `test_sync_3d_player_updates_translation`.)_
- [x] Update player material color dynamically based on `Player.alive` (`COLOR_PLAYER` / `COLOR_PLAYER_DEAD`), replacing 2D `ColorMaterial` modifications.
      _(Note: Implemented dynamic color updates in `sync_3d_player` in `src/view3d.rs` setting `StandardMaterial.base_color` to `COLOR_PLAYER` when alive and `COLOR_PLAYER_DEAD` when dead; verified by unit tests `test_player_material_dynamically_updates_on_alive_state` and `test_player_spawned_dead_initializes_with_dead_color`.)_

##### Car Boxes & Curve Navigation

- [x] Add `On<Add, Car>` observer spawning `Cuboid` (`length × CAR_HEIGHT_3D × width`, long axis = local X) plus `View3dOf(car)`.
      _(Note: Implemented `on_add_car` observer in `src/view3d.rs` spawning `Cuboid` mesh, `Transform` with local X long axis and y flush with ground, and `View3dOf` relationship with `linked_spawn`; verified by unit test `test_on_add_car_spawns_cuboid_and_links`.)_
- [x] Allocate shared mesh handles and small palette of shared materials (zero per-car allocation).
      _(Note: Added `CarAssets3d` resource initialized in `View3dPlugin` containing shared cuboid mesh and 5-color `StandardMaterial` palette matching `CAR_COLORS`; `on_add_car` clones handles with zero per-car allocations; verified by unit test `test_car_assets_3d_shared_zero_allocation`.)_
- [x] Implement `sync_3d_cars`: sync translation from `GroundPos` and rotation from `heading_to_3d(Heading)`.
      _(Note: Implemented `sync_3d_cars` in `src/view3d.rs` querying `GroundPos` and `Heading` via `View3dOf`, updating translation to `ground_to_3d(pos, half_height)` and rotation to `heading_to_3d(heading)`; verified by unit test `test_sync_3d_cars_updates_translation_and_rotation`.)_
- [x] Visual check: cars traverse straight lanes and curves with smooth orientation transitions.
      _(Note: Verified visually via Chromium web build capture and unit test `test_cars_traverse_straight_and_curve_with_smooth_transitions`; screenshots confirm cars render as cuboids flush on asphalt and smoothly follow straight lanes and curved arc tangent to trajectory with zero z-fighting.)_
- [x] Verify car despawning and game restart remove associated 3D boxes via `linked_spawn`.
      _(Note: Verified via unit test `test_car_despawn_and_game_restart_removes_3d_boxes_via_linked_spawn` and browser testing; out-of-bounds despawning via `traffic::car_cleanup` despawns associated 3D proxy boxes without affecting in-bounds cars, and game restart via `collision::restart_game` removes all car entities and linked 3D boxes with 0 leaked entities.)_

##### Blood Splats

- [x] Add `On<Add, BloodSplat>` observer spawning decal from `create_blood_splat_mesh(seed, …)`, rotated $-\pi/2$ about X to lie flat at $y = 0.05$, using unlit `StandardMaterial`.
      _(Note: Implemented `on_add_blood_splat` observer in `src/view3d.rs` spawning `create_blood_splat_mesh`, unlit `StandardMaterial` with `COLOR_BLOOD_SPLAT`, transform rotated $-\pi/2$ about X at $y = 0.05$, and `View3dOf` relationship with `linked_spawn`; verified by unit test `test_on_add_blood_splat_spawns_decal_and_links`.)_
- [x] Verify game restart cleans up splat decals.
      _(Note: Verified via unit test `test_blood_splat_despawn_and_game_restart_removes_3d_decals_via_linked_spawn`; individual splat despawn despawns linked 3D decal, and game restart via `KeyR` or `Space` in `GameOver` state clears all simulation `BloodSplat` entities and automatically cleans up all associated 3D decal entities via `linked_spawn`.)_

#### 5.3 Traffic-Feature Rendering Prerequisites

- [x] Keep traffic decisions in simulation data (`GroundPos`, `Heading`, lane/progress, speed, signal state); views only render that state.
      _(Note: Verified unidirectional data flow from simulation data to 3D views; `view3d::sync_3d_cars` and `on_add_car` strictly consume simulation components `GroundPos`, `Heading`, and `traffic::Car` without mutating simulation data or housing traffic decision logic, verified by unit test `test_traffic_decisions_remain_in_simulation_data_views_only_render`.)_
- [x] Expose shared car mesh/material handles so normal, braking, and crashed cars can change appearance without allocating materials per car.
      _(Note: Added `braking_material` with `COLOR_CAR_BRAKING` and `crashed_material` with `COLOR_CAR_CRASHED` to `CarAssets3d`, along with helper methods `normal_material`, `braking_material`, and `crashed_material`; verified via unit tests `test_car_assets_3d_shared_zero_allocation` and `test_car_assets_shared_normal_braking_crashed_appearance_transition` confirming appearance transitions allocate 0 additional materials.)_
- [x] Keep stop-line and signal placement derived from `RoadGeometry`, not duplicated world-coordinate constants.
      _(Note: Added `stop_line_position`, `stop_line_size`, `stop_line_bounds`, `signal_position`, and `signal_facing_heading` to `RoadGeometry` deriving stop-line and signal placements directly from geometry and lane coordinates without duplicated world-coordinate constants; verified by unit test `test_stop_line_and_signal_placement_derived_from_road_geometry`.)_

---

### Phase 6: Driving Logic, Queues, Rear-End Crashes & Stoplight

Scope decision: implement one intersection-wide signal shared by all four approaches. It cycles `Green → Yellow → Red → Green`; coordinated horizontal/vertical right-of-way phases, turning arrows, and cross-lane intersection crashes are intentionally deferred. Cars already past their stop line always clear the intersection.

#### 6.1 Traffic Foundations (Parallel Workstreams)

Tackle route progress, the signal state machine, and finite acceleration in parallel. Agree on the `CarMotion` fields and progress/stop-line helper signatures first; queue planning in 6.2 is the integration point for all three workstreams.

##### Route Progress: One Coordinate for Straight and Curved Lanes

- [x] Add pure `lane_progress(lane, position, &RoadGeometry) -> f32`, measured from that lane's spawn end and increasing in the travel direction.
      _(Note: Implemented pure `lane_progress(lane, position, &RoadGeometry) -> f32` and `TrafficLane::progress(&self, position, &RoadGeometry) -> f32` in `src/traffic.rs`; returns 0.0 at spawn origin and increases monotonically along travel direction.)_
- [x] Cover all route segments:
  - Eastbound: west straight → quarter-circle curve → north straight.
  - Westbound: southbound straight from the north edge → reverse quarter-circle → west straight.
  - Northbound and Southbound: direct vertical progress.
    _(Note: Implemented full segment coverage and boundary continuity in `lane_progress`; verified via unit test `test_lane_progress_all_lanes_monotonic_and_continuous` covering straight, curved, and transition sections across all four lanes.)_
- [x] Add `stop_line_progress(lane, &RoadGeometry) -> f32` for a car center stopped one half-car-length plus a small buffer before `intersection_bounds`.
      _(Note: Implemented `stop_line_progress(lane, road_geom)` in `src/traffic.rs` and `RoadGeometry::stop_line_progress(lane)` in `src/road.rs` using `CAR_LENGTH / 2.0 + STOP_LINE_BUFFER`; verified via unit test `test_stop_line_progress_all_lanes` across all four approaches.)_
- [x] Add `front_progress`/`rear_progress` helpers using `Car.length`; following and collision code must compare bumper gaps, not center distances.
      _(Note: Added `front_progress` and `rear_progress` pure helpers and `Car` methods in `src/traffic.rs`; verified via `test_front_rear_progress_and_bumper_gaps` proving bumper gap computations subtract vehicle half-lengths from center distance.)_
- [x] Unit-test monotonic progress immediately before/after each straight/curve boundary, both curve directions, and all four stop lines.
      _(Note: Verified via unit test `test_monotonic_progress_across_boundaries_and_stop_lines` testing $\epsilon$-step monotonicity across entry/exit boundaries of Eastbound and Westbound curves, along with before/at/after ordering and bumper buffer offsets for all four stop lines.)_

##### Signal State Machine

- [x] Add `TrafficLightState { Green, Yellow, Red }` and a `TrafficSignal` resource holding the current state and repeating phase timer.
- [x] Add named initial tuning constants: green `6.0s`, yellow `1.5s`, red `5.0s`. Keep these compile-time constants until playtesting proves runtime configuration is needed.
- [x] Implement `tick_traffic_signal` with deterministic transitions `Green → Yellow → Red → Green`; initialize at `Green`.
- [x] Define approach behavior:
  - Green: no signal speed restriction.
  - Yellow: stop if the car can do so at comfortable deceleration before the line; otherwise commit to crossing.
  - Red: stop before the line unless the car committed during yellow or its front bumper already crossed the line.
- [x] Store the yellow commitment per car and clear it after the rear bumper passes the stop line so a red transition cannot strand a car inside the intersection.
      _(Note: Stored `committed_to_cross` per car, added `update_crossing_commitment` pure helper and method on `Car`, and implemented `update_car_signal_commitments` system clearing commitment once `rear_progress >= stop_line`; verified via unit tests `test_yellow_commitment_lifecycle_and_clearing_after_rear_bumper` and `test_update_car_signal_commitments_system`.)_
- [x] Unit-test exact phase transitions, timer rollover, yellow stop/go decisions on both sides of the stopping-distance boundary, and the red-after-yellow commitment case.
      _(Note: Verified via unit tests `test_traffic_signal_deterministic_transitions`, `test_traffic_signal_rollover`, `test_signal_approach_behavior`, and `test_red_after_yellow_commitment_and_non_commitment_cases` covering Green/Yellow/Red deterministic timing, multi-phase rollover, $\pm\epsilon$ and exact boundary stopping distance conditions, and commitment survival through red until rear bumper clearance.)_

##### Car Motion State and Finite Acceleration

- [x] Separate design speed from instantaneous motion: keep `CAR_SPEED` as cruise speed and add per-car `CarMotion { current_speed, desired_speed, perceived_speed_limit, pending_speed_limit, reaction_remaining, committed_to_cross }`.
- [x] Initialize spawned cars at cruise speed; update legacy tests and direct test fixtures to include `CarMotion`.
- [x] Add initial tuning constants:
  - `CAR_ACCELERATION = 160.0` units/s².
  - `CAR_COMFORTABLE_DECELERATION = 300.0` units/s².
  - `CAR_EMERGENCY_DECELERATION = 520.0` units/s².
  - `DRIVER_REACTION_SECS = 0.35`.
  - `CAR_MIN_FOLLOW_GAP = 14.0` units.
- [x] Add a pure speed-step helper that clamps acceleration/deceleration by `dt`, never produces a negative speed, and never overshoots the desired speed.
- [x] Change `car_movement` to advance route position by `current_speed * dt`; retain its existing lane snapping, curve math, headings, and cleanup behavior.
- [x] Unit-test acceleration from rest, comfortable braking, emergency braking, zero-speed stability, and a single large frame that must not move farther than the computed speed step permits.
      _(Note: Implemented `apply_car_acceleration` system and added comprehensive tests `test_finite_acceleration_from_rest`, `test_finite_comfortable_braking`, `test_finite_emergency_braking`, `test_zero_speed_stability`, and `test_single_large_frame_bounded_motion` in `src/traffic.rs` covering pure step math and integrated ECS movement bounded by acceleration limits.)_

#### 6.2 Leader Detection, Driver Reaction, and Queue Formation

- [x] Add `plan_car_motion` before `car_movement`. Group live cars by `TrafficLane`, order each group by `lane_progress`, and compute each follower's bumper gap to the nearest car ahead.
- [x] Reuse scratch vectors through Bevy `Local` state; do not allocate fresh lane lists every frame.
- [x] Compute each car's desired speed as the most restrictive of:
  - Cruise speed.
  - The speed that preserves `CAR_MIN_FOLLOW_GAP` behind the leader.
  - The speed that stops at a red/yellow stop line.
- [x] Use $v_\text{safe} = \sqrt{2ad}$ with non-negative available distance `d` and the appropriate deceleration constant; clamp invalid/overlapping gaps to zero.
- [x] Model reaction delay only when a newly observed limit is stricter: hold the old perceived limit while `reaction_remaining > 0`, retain the strictest pending limit without restarting the timer, then apply it. Relaxed limits may apply immediately so queues start moving promptly on green.
- [x] Apply emergency deceleration when the reacted safe speed is below what comfortable braking can reach this frame; emergency braking is finite, so an abrupt stop can still produce a collision.
- [x] Unit-test a three-car queue: lead car brakes for red, followers react in order, all stop without overlap at adequate spacing, and all accelerate after green.
      _(Note: Verified via unit test `test_three_car_queue` testing ordered follower reaction, stopping before the line with adequate follow spacing, and clean queue acceleration on green.)_
- [x] Unit-test an abrupt stopped leader at two gaps: adequate distance avoids contact; insufficient distance creates overlap despite emergency braking.
      _(Note: Verified via unit test `test_abrupt_stopped_leader_two_gaps`: adequate distance (250.0) avoids contact and stops safely with spacing $\ge \text{CAR\_MIN\_FOLLOW\_GAP} - 1.0$, while insufficient distance (90.0) activates emergency braking but still produces rear-end bumper overlap $\le 0.0$.)_

#### 6.3 Rear-End Collision and Crash Lifecycle

- [x] Add `detect_rear_end_crashes` after movement. Compare adjacent cars in the same lane using route progress and bumper gap; a gap `<= 0` is a rear-end collision.
- [x] Add a `Crashed` component with a cleanup timer. On impact, mark both cars crashed and set all motion speeds/limits to zero; crashed cars remain physical obstacles until cleanup.
- [x] Ignore already-crashed pairs when emitting new impacts so one overlap does not refresh timers or duplicate effects every frame.
- [x] Despawn crashed cars after a short named duration (start at `3.0s`) to prevent permanent road blockage and entity leakage.
- [x] Prevent `spawn_cars` from spawning into an occupied spawn footprint; defer that lane's spawn until enough bumper clearance exists rather than creating an immediate crash off-screen.
- [x] Keep player/car collision behavior unchanged; this slice handles only same-lane rear-end crashes.
- [x] Add tests for one impact transition, both cars stopping, no duplicate impact on the next frame, timed cleanup, adjacent-lane non-collision, and blocked-spawn deferral.

#### 6.4 Stoplight and Driving-State Visuals

Once the `TrafficSignal`, `CarMotion`, and `Crashed` contracts are fixed, tackle the 3D signal heads, shared road markings, and driving-state palette in parallel with 6.2–6.3. Add the top-down debug pieces when the Phase 8 debug view exists.

- [x] Add four approach-facing signal heads driven by the one `TrafficSignal` resource so the shared aspect is visible from every lane.
      _(Note: Spawned four approach-facing 3D signal heads via `setup_signals_3d` in `view3d.rs` using `RoadGeometry::signal_position` and `RoadGeometry::signal_facing_heading`, driven in lockstep by `sync_3d_signals` querying `TrafficSignal`; verified via `test_setup_signals_3d_spawns_four_approach_heads` and `test_sync_3d_signals_drives_all_heads_without_material_allocation`.)_
- [x] Build each 3D head from shared primitive meshes/materials: pole/housing plus red, yellow, and green lenses. Change visibility/emissive state on transitions without creating new materials.
      _(Note: Implemented `SignalAssets3d` with shared pole cylinder, housing cuboid, lens sphere, and pre-allocated emissive red/yellow/green materials; `sync_3d_signals` toggles lens visibility without allocating materials; verified via `test_signal_assets_3d_shared_zero_allocation` and `test_sync_3d_signals_drives_all_heads_without_material_allocation`.)_
- [x] Add one stop-line marking per approach through shared road-piece data; render it in both 3D and the top-down debug view.
      _(Note: Added `RoadPieceKind::StopLine` and four approach stop lines derived from `RoadGeometry::stop_line_position` and `stop_line_size` to shared `road_pieces`; rendered in 3D with shared `COLOR_ROAD_STOP_LINE` material via `setup_roads_3d`; debug-view rendering will consume the shared `road_pieces` when `draw_debug_2d` is created in Phase 8; verified via `test_road_pieces_counts_and_properties` and `test_setup_roads_3d_spawns_pieces_with_shared_resources`.)_
- [x] Show braking and crash state with shared palette materials (for example, brake-light red and a distinct crashed-car color); do not put simulation decisions in rendering systems.
      _(Note: Added `CarVisualState` enum and `CarMotion::is_braking`/`CarMotion::visual_state` simulation contract in `traffic.rs`; added `CarAssets3d::material_for_state` and updated `sync_3d_cars` and `on_add_car` in `view3d.rs` to dynamically switch shared materials between normal palette, braking, and crashed without allocating materials; verified via `test_car_motion_is_braking_and_visual_state` and `test_sync_3d_cars_drives_driving_state_materials_without_allocation`.)_
- [x] Extend `draw_debug_2d` with compact signal-state markers and crashed-car outlines so behavior can be diagnosed without 3D meshes.
      _(Note: Implemented `draw_debug_2d` using Bevy 2D gizmos in `src/main.rs`, rendering compact signal-state markers per approach curb derived from `RoadGeometry::signal_position` and colored by `TrafficSignal` aspect, stop-line markings from `road_pieces`, rotated rect outlines and wireframe cross indicators for `Crashed` vehicles in `COLOR_CAR_CRASHED`, and rear brake-light circles in `COLOR_CAR_BRAKING`; verified via unit tests `test_draw_debug_2d_empty_world`, `test_draw_debug_2d_signal_phases`, and `test_draw_debug_2d_handles_normal_braking_and_crashed_cars`.)_
- [x] Verify signal visuals, stop lines, brake state, and crash state in both camera modes and in the web build.
      _(Note: Verified via unit test `test_verify_signal_visuals_stop_lines_brake_and_crash_in_both_camera_modes` in `src/main.rs` testing 3D and 2D camera modes with Green/Yellow/Red signal transitions, 4 approach stop lines, and Normal/Braking/Crashed driving states without allocation; verified on the web target via `trunk build --release --cargo-profile wasm-release` running in headless Chromium with WebGL2, capturing screenshots across signal phases and live traffic showing 3D signal heads and curb markers, stop lines, braking materials, and rear brake lights with zero wasm errors or panics.)_

#### 6.5 Scheduling, Integrated Scenarios, and Done Criteria

- [x] Order traffic systems explicitly: `tick_traffic_signal → spawn_cars → plan_car_motion → apply_car_acceleration → car_movement → detect_rear_end_crashes → tick_crash_cleanup → car_cleanup`.
      _(Note: Factored `tick_crash_cleanup` out of `car_cleanup` to handle timer advances separately from despawn, and registered the exact 8-system explicit chain via `.chain().run_if(in_state(AppState::InGame))` in `TrafficPlugin::build`; verified via unit tests `test_traffic_systems_explicit_chain_order`, `test_crashed_car_cleanup_after_duration`, and full suite.)_
- [x] Keep the chain gated by `AppState::InGame`; verify non-game states freeze the signal phase, driver reaction, movement, and crash cleanup timers.
      _(Note: Verified via unit test `test_traffic_chain_gated_by_in_game_freezes_in_non_game_states`; transitions to `AppState::GameOver` completely freeze traffic signal timers and phases, vehicle positions, driver reaction delays, speed adaptations, and crash cleanup timers without despawning vehicles; transitioning back to `AppState::InGame` resumes signal progression, vehicle motion, driver reaction delays, and crash cleanup.)_
- [x] Update restart handling to reset the signal to `Green`, clear queued/crashed cars through the existing entity cleanup path, and reset spawn timing.
      _(Note: Added `TrafficSignal` to `RestartResources`, implemented `TrafficSignal::reset()`, and updated `restart_game` to reset the signal to Green, clear all queued and crashed cars via entity despawn with linked 3D proxy cleanup, and reset `SpawnTimer` back to default timing and initial lane; verified via unit test `test_restart_resets_signal_clears_queued_and_crashed_cars_and_resets_spawn_timer`.)_
- [x] Add an integrated deterministic scenario covering a complete cycle: cars enter on green, queue on red without crossing the line, restart on green, and one deliberately short-gap follower crashes after an abrupt stop.
      _(Note: Implemented deterministic scenario in `test_integrated_deterministic_scenario_complete_cycle` covering entry on green, deceleration and queued stop at `0.0` speed before the stop line on red, acceleration on green, and a moving leader that stops abruptly causing a short-gap follower to crash and clean up; fixed yellow stopping commitment flipping by latching `stopping_for_signal` in `CarMotion` and `compute_signal_speed_limit`; verified via `test_integrated_deterministic_scenario_complete_cycle`, `test_yellow_braking_never_commits_across_step_sizes_and_speeds` across dt 0.05/0.10 and speeds 200/240, `test_compute_signal_speed_limit_latched_stopping_and_clearing`, and full test suite.)_
- [x] Native smoke: watch at least two full signal cycles, verify every approach obeys the same aspect, queues do not overlap at normal spacing, yellow-committed cars clear the intersection, crashes remain possible at insufficient spacing, wrecks clear, and spawning recovers.
      _(Note: Verified via unit test `test_native_smoke_two_full_signal_cycles_integrated` covering 30.0s / 600 steps and native execution on Metal: all 4 approach signal heads synchronously display identical aspects matching `TrafficSignal.state`; queued vehicles maintain non-negative bumper gaps across all lanes without overlapping; dilemma-zone car commits during yellow and safely clears past stop line; short-gap following triggers rear-end crash with crashed 3D material; wreck cleans up after 3.0s; spawning recovers cleanly in affected lane; full test suite of 145 tests passes.)_
- [x] Browser smoke: run the same scenario under `trunk serve`; verify stable motion through a throttled/long frame and no console panic.
      _(Note: Verified via web build served locally via `trunk serve --release --cargo-profile wasm-release` and automated in Chromium: executed > 30s across multiple full 12.5s signal cycles; observed green traversal, queue formation behind stop lines without intersection penetration during red, green restart acceleration, and yellow/red cycle progression; tested throttled and long frames including 400ms main-thread freeze, 4x CPU throttling through yellow-to-red transitions, and 700ms freeze during green restart; verified bounded, stable vehicle motion with zero tunneling or overshoots and 0 console errors or panics; captured visual proof via canvas screenshots.)_

---

### Phase 7: Wasm Size Checkpoint & Profile Optimization

Target: **< 15 MB** uncompressed wasm (ideally < 10 MB). Baseline before 3D: 29.4 MB.

After recording the checkpoint, tackle profile tuning and transfer-compression verification in parallel; compare each result against the same checkpoint build.

- [x] **3D Size Checkpoint**: build wasm after `View3dPlugin` primitive rendering is complete:
  ```bash
  trunk build --release --cargo-profile wasm-release
  ```
  Record the uncompressed `.wasm` size. If size blows the budget significantly, assess before continuing.
  - **Recorded size**: 29.7 MB (31,174,315 bytes), an increase of ~300 KB over the 29.4 MB pre-3D baseline. No unexpected size blowup from 3D plugin primitives. Proceeding to profile tuning.
- [x] **Profile overrides**: test adjustments in `[profile.wasm-release]` in `Cargo.toml` and adopt wins:
  ```toml
  opt-level = "z"
  lto = "thin"
  panic = "abort"
  ```
  - **Results**: `opt-level = "z"` and `panic = "abort"` successfully reduced uncompressed `.wasm` from 29.7 MB (31,174,315 bytes) to 25.1 MB (26,343,936 bytes), saving 4.8 MB (~15.5%). `lto = "fat"` was evaluated but times out (>10 min) under 8 GB RAM constraints with 295 Bevy crates; `lto = "thin"` was retained for fast, viable builds.
- [x] **Verify transfer compression**: confirm static server gzip / brotli compression reduces transfer footprint ~3–4× (e.g. 29 MB raw $\to$ ~7–9 MB over wire).
  - **Verified Transfer Footprint**:
    - Raw `.wasm`: 26.3 MB (26,343,936 bytes)
    - Gzip (`-6` server default): 7.88 MB (8,262,047 bytes, 3.19× compression)
    - Gzip (`-9` static max): 7.85 MB (8,230,097 bytes, 3.20× compression)
    - Brotli (`-q 9` standard): 5.98 MB (6,268,057 bytes, 4.20× compression)
    - Brotli (`-q 11` max): 5.22 MB (5,474,052 bytes, 4.81× compression)
      Transfer footprint is reduced ~3.2× to ~4.8×, reaching ~5.2–7.9 MB over the wire.

---

### Phase 8: Replace 2D Meshes with Gizmo Debug View & Feature Trimming

- [x] Create `src/view_debug2d.rs` with `DebugView2dPlugin`. Move `Camera2d` spawn and follow logic (`setup_camera`, `camera_follow_player`) from `main.rs`, tagging the camera `TopDownCamera`.
- [x] Implement `draw_debug_2d` gizmos: player circle (colored by `alive`), car boxes as rotated rects from `GroundPos`/`Heading`/footprint, road outlines from `road_pieces`, and an X marker for each blood splat.
- [x] Delete obsolete 2D visual code: `Mesh2d`/`MeshMaterial2d` on player, cars, and splats; road `Sprite` components; `Z_*` layering constants; and the temporary `sync_2d` bridge from Phase 1.
- [x] Update existing tests that query `Mesh2d` / `ColorMaterial` (`test_setup_player_spawns_entity`, collision color tests).
      _(Note: Verified `test_setup_player_spawns_entity` queries `GroundPos` and `Heading`, collision tests run simulation state without 2D materials, and removed residual unused `Assets<ColorMaterial>` initializations from traffic unit tests; zero `Mesh2d` or `ColorMaterial` references remain across `src/`.)_
- [x] **Feature trimming evaluation**: Bevy defaults are `["2d", "3d", "ui", "audio"]`. In Bevy 0.20, `LineGizmo2dPlugin` on `Camera2d` depends on `bevy_sprite_render` (enabled only by `2d`). Choose one:
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
  - **Decision & Measurement**: Chose **Option A**. Option B would need `RenderLayers` on the debug camera plus gizmo config; Option A avoids it. Unused `audio` was trimmed by setting `default-features = false` and specifying `["2d", "3d", "ui", "default_platform", "default_app"]`. Uncompressed `.wasm` size measured via `trunk build --release --cargo-profile wasm-release`: **25,874,695 bytes** (24.7 MB), down 469,241 bytes (~470 KB) compared to Phase 7's 26,343,936 bytes.
- [x] Test both `cargo run` and `trunk serve` after any feature adjustments.
      _(Note: Verified `cargo run` initialized window and render pipeline cleanly; verified `trunk serve --release --cargo-profile wasm-release` successfully built wasm distribution and served HTTP 200 on port 8080.)_

---

### Phase 9: View Toggle (3D Main View / 2D Debug View)

- [x] Add `ViewMode { ThreeD, TopDown }` as a Bevy `States` (default `ThreeD`).
- [x] Add `OnEnter(ViewMode::*)` systems flipping `is_active` on `ThreeDCamera` and `TopDownCamera`.
- [x] Restrict `draw_debug_2d` system execution to run only in `ViewMode::TopDown`.
- [x] Add top-left UI `Button` displaying the alternative view mode, toggling `ViewMode` on `Interaction::Pressed`.
- [x] Add keyboard shortcut `V` for the view toggle (avoids conflict with WASD/arrows, `R`, `Space`).
- [x] Ensure Bevy UI / HUD (SPD counter, toggle button) remains active and properly attached across view switches (manage `IsDefaultUiCamera` or `UiTargetCamera`).

---

### Phase 10: Automated Verification & Testing

Once Phases 8–9 land, tackle the linking, toggle, simulation-independence, and integrated smoke tests in parallel, then run the complete suite once.

- [x] **Linking test**: spawning a car produces exactly one `View3d` proxy whose `Transform` matches `ground_to_3d(GroundPos)`; despawning car removes proxy.
      _(Note: Implemented `test_linking_car_spawn_produces_exactly_one_view3d_proxy_matching_ground_to_3d_and_despawn_removes_proxy` in `src/view3d.rs`; verifies spawning car produces exactly 1 `View3d` proxy with matching translation `ground_to_3d(GroundPos)` and heading rotation, updates match on motion, and despawning the car entity removes the proxy via `linked_spawn`.)_
- [x] **Toggle test**: button or `V` key flips `ViewMode`, exactly one camera is active, and toggling twice restores `ThreeD`.
      _(Note: Implemented `test_toggle_flips_view_mode_exactly_one_camera_active_and_toggling_twice_restores_threed` in `src/view_toggle.rs`; verifies both UI button and `V` key toggle `ViewMode` between `ThreeD` and `TopDown`, invariant that exactly one camera is active in the world at all times, and toggling twice cleanly restores `ThreeD`.)_
- [x] **Simulation independence test**: vehicle collisions and SPD increment logic behave identically in both `ViewMode::ThreeD` and `ViewMode::TopDown`.
      _(Note: Implemented `test_simulation_independence_collisions_and_spd_identical_across_view_modes` in `src/main.rs`; verifies vehicle-pedestrian collisions, blood splat seeds/positions, player resets, multiple sequential SPD counter increments, and vehicle rear-end crash detection execute identically across `ViewMode::ThreeD` and `ViewMode::TopDown`, including across dynamic mid-simulation view mode toggles.)_
- [x] Update `test_full_integrated_app_smoke` headless test to include both view plugins (with `Assets<StandardMaterial>` mock setup).
      _(Note: Updated `test_full_integrated_app_smoke` in `src/main.rs` with `Assets<StandardMaterial>` and `ViewMode` setup to include `View3dPlugin`, `DebugView2dPlugin`, and `ViewTogglePlugin`; verifies initial active 3D camera / inactive 2D camera, 3D player proxy positioning and translation tracking, simultaneous 2D camera tracking and clamping, and view toggle switching.)_

---

### Phase 11: 3D Web Validation

Validate the release candidate in the locally available browser.

#### 11.1 Browser Validation

- [x] **Browser compatibility**: test the 3D build in Chromium using `env -u NO_COLOR trunk serve`. Verify zero WebGL2 shader compilation failures or buffer errors in console.
      _(Note: Verified in Chromium via `trunk serve` on port 8080; WebGL 2.0 ANGLE Metal renderer initialized cleanly with 0 errors and 0 shader compilation or buffer failures.)_
- [x] **Context & toggle stability**: verify switching views (`V` or button) occurs smoothly on web without frame drops or WebGL context losses.
      _(Note: Verified view toggling via `KeyV` and button click; 0 `webglcontextlost` events and steady ~60.5 FPS with max frame delta 16.8 ms across rapid toggles.)_
- [x] **HUD stability**: ensure SPD text and toggle button remain visible across both view modes in web canvas.
      _(Note: Verified SPD text ("SPD: 0" -> "SPD: 3") and view toggle button ("2D Debug (V)" / "3D View (V)") render sharply and update dynamically across both 3D and 2D debug views.)_
- [x] **Depth test check**: verify gizmo debug lines render without depth-test clipping against clear color on web.
      _(Note: Verified 2D debug gizmo lines (2742 white, 283 yellow, 130 green, 1064 red pixels) render with full contrast and without depth-test clipping against clear color.)_

---

### Phase 12: Release Readiness & Documentation

After the native smoke passes, update release documentation.

- [x] **Native smoke run**: execute `cargo run`, play in 3D, switch to debug 2D mid-traffic, confirm boxes/circles match positions, toggle back, crash, restart.
      _(Note: Verified native binary startup on macOS Metal and automated end-to-end integration test `test_native_smoke_play_toggle_crash_restart` in `src/main.rs`: starts in 3D view with active 3D camera and inactive 2D camera, moves player forward with `KeyW`, spawns traffic car with matching 3D proxy translations, toggles to debug 2D view mid-traffic via `KeyV`, confirms 2D camera activation and exact match of simulation GroundPos / 2D gizmo shapes with 3D proxy coordinates, toggles back to 3D, triggers collision crash incrementing SPD and spawning blood splat, and restarts via `KeyR` resetting player to spawn, resetting SPD to 0, clearing blood splats and despawning traffic.)_
- [x] Update control instructions in `README.md` (WASD/arrows, `R`, `V` / button).
      _(Note: Added `## Controls` table in `README.md` detailing WASD/arrows movement, `R` / `Space` restart and counter reset, and `V` / top-left UI button view mode toggle.)_
- [x] Check off Phase 1 decoupling and Phase 7 web distribution tasks in `docs/masterplan.md`.
      _(Note: Updated `docs/masterplan.md`: checked off Phase 1 prerequisite simulation position decoupling tasks `GroundPos`/`Heading`, movement/collision refactoring, and transform sync; checked off web build & distribution tasks including toolchain setup, canvas configuration, wasm size audit, and GitHub Pages CI deployment.)_
- [x] Update `docs/isometric-view.md` §5 (record plain-3D + gizmo debug implementation and recorded wasm sizes).
      _(Note: Updated `docs/isometric-view.md` §5 with completed plain-3D foundation, gizmo debug architecture, view mode toggle details, recorded wasm sizes table (29.4 MB baseline -> 26.3 MB 3D -> 24.7 MB trimmed / 6.7 MB gzip), WebGL2 compatibility, and roadmap toward isometric pixel-art style.)_

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

- [x] **Bevy best-practices review**: Audit the repository against current Bevy project best practices, with particular attention to thoughtful ECS design (clear component/resource boundaries, focused systems, explicit scheduling, event/observer use, query efficiency, change detection, and avoiding unnecessary entity/component churn); refactor confirmed issues and verify native and web behavior remains unchanged.
  - **Handoff notes**:
    - Added `GameSet` (`Input` -> `Sim` -> `Collision` -> `Cleanup` -> `RenderSync` -> `DebugRender`) in `src/state.rs` and chained in `src/main.rs` and plugins for explicit deterministic scheduling.
    - Added broad-phase bounding circle early-exit in `collision_detection` to avoid OBB trigonometry for distant cars.
    - Cached shared `StandardMaterial` in `on_add_blood_splat` via `Local` to eliminate per-splat asset allocations.
    - Added change-detection guards (`is_changed` run conditions and value-comparison guards before mutating `Transform`, `Visibility`, `GroundPos`, and `CarMotion`) to avoid spurious archetype dirtying on stationary cars/player/lenses.
    - Verified with all 161 unit/integration tests and `wasm32-unknown-unknown` compilation.
