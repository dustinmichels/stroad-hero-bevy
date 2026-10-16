# Changelog

## October 15, 2025

### 3D Main View & Visuals

- Introduced `View3dPlugin` rendering gameplay in 3D using plain primitives: capsule player, cuboid cars, flat plane road segments, and blood splat decals.
- Set up perspective `Camera3d` facing north with follow-and-clamp tracking, directional sunlight with a single shadow cascade, and ambient fill light for WebGL2 compatibility.
- Implemented `View3dOf` entity relationships with `linked_spawn` so 3D proxy entities mirror simulation state and automatically despawn on entity cleanup.
- Added four approach-facing 3D signal heads with emissive lenses and shared materials for normal, braking, and crashed car states without per-entity allocations.

### Simulation Decoupling & Architecture

- Decoupled simulation from rendering transforms: all gameplay systems now operate on 2D ground coordinates (`GroundPos(Vec2)` and `Heading(f32)`).
- Normalized vehicle footprints and refactored player movement, car traversal, collision detection, and splats to pure simulation data.
- Established explicit system scheduling via `GameSet` pipeline (`Input` → `Sim` → `Collision` → `Cleanup` → `RenderSync` → `DebugRender`).
- Added broad-phase circle early exits to collision checks, cached decals, and change-detection guards on motion and transforms to prevent unnecessary ECS archetype churn.

### Traffic Dynamics & Intersection Signal

- Added route-aware `lane_progress` tracking covering straight segments and curved turns with stop-line bumper offsets.
- Implemented `TrafficSignal` state machine cycling Green → Yellow → Red with dilemma-zone crossing commitments and yellow stopping latching.
- Added finite vehicle acceleration (`CarMotion`), comfortable and emergency deceleration, driver reaction delays, safe following gap calculation ($v = \sqrt{2ad}$), and multi-car queueing.
- Added same-lane rear-end collision detection, `Crashed` status with 3.0s despawn timers, and spawn-blocking deferral.

### 2D Gizmo Debug View & Toggle

- Created `DebugView2dPlugin` replacing legacy 2D sprites and meshes with lightweight Bevy gizmos (player circle, vehicle boxes, road outlines, signal curb markers, splat crosses, and brake lights).
- Added `ViewMode` state machine (`ThreeD` / `TopDown`) toggling active cameras via top-left UI button and the `V` hotkey, keeping Bevy UI/HUD persistent across views.
- Removed obsolete 2D sprite/mesh components, temporary sync bridges, and unused 2D color materials.

### Web Build & Performance Optimization

- Added Trunk toolchain configuration and `index.html` canvas binding (`#game`) running at 800×800 with browser default key handling suppressed.
- Configured GitHub Pages deployment workflow (`.github/workflows/web.yml`) and itch.io release packaging.
- Optimized release profile (`opt-level = "z"`, `lto = "thin"`, `panic = "abort"`) and trimmed unused Bevy `audio` feature, dropping raw wasm from 29.4 MB to 24.7 MB (~5.2–7.9 MB over wire with Brotli/Gzip).
- Validated WebGL2 rendering and state transitions across Chromium and native Metal with zero console errors or context losses.
