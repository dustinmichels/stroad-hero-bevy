# MVP Implementation Plan: 2D Road Intersection & Player

## 1. Project Setup & Core Configuration

- [x] Verify Bevy window and 2D camera setup (`Camera2d`) in `src/main.rs`.
- [x] Define global game constants and coordinate system:
  - [x] Window dimensions and virtual world bounds (e.g., 800x800).
  - [x] Grid step size / coordinate scaling for road alignment.
  - [x] Movement speeds for player and cars.
  - [x] Vehicle spawn intervals and dimensions.
- [x] Establish game states (`AppState::InGame`, `AppState::GameOver`) to manage gameplay and death/reset cycles.

## 2. World Representation & Roads (Lines)

- [x] Define road geometry for a single intersection:
  - [x] Horizontal road centered along Y-axis (e.g., $y = 0$).
  - [x] Vertical road centered along X-axis (e.g., $x = 0$).
  - [x] Optional background grid visualization (faint grid lines/points).
- [x] Implement road rendering system:
  - [x] Render road lines using Bevy Gizmos (`gizmos.line_2d`) or static 2D line meshes/rectangles for lanes and centerlines.
  - [x] Ensure roads are layered beneath player and vehicles in Z-ordering (e.g., $z = 0.0$ for roads, $z = 1.0$ for cars, $z = 2.0$ for player).

## 3. Player Character (Circle) & Movement

- [x] Define `Player` component with player attributes (radius, speed, alive state).
- [x] Implement player spawning system:
  - [x] Spawn player as a circle primitive using `Mesh2d` with `Circle` and `ColorMaterial` (e.g., distinct bright color like green or blue).
  - [x] Set initial spawn location safely off the road or at an intersection corner.
- [x] Implement arrow key movement system:
  - [x] Read `ButtonInput<KeyCode>` for `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`.
  - [x] Compute directional delta and update player `Transform.translation` scaled by delta time (`time.delta_secs()`).
  - [x] Clamp player coordinates within window / playfield boundaries.

## 4. Traffic System (Rectangular Cars)

- [x] Define vehicle components and resources:
  - [x] `Car` component containing speed, direction (`Vec2`), and bounding dimensions (width, height).
  - [x] `SpawnTimer` resource controlling periodic car generation across lanes.
- [x] Implement car spawning logic:
  - [x] Off-screen spawn origins (e.g., left off-screen $x = -W/2 - \text{margin}$ heading East, top off-screen $y = H/2 + \text{margin}$ heading South).
  - [x] Instantiate cars as rectangular primitives (`Mesh2d` with `Rectangle` or scaled 2D quads).
  - [x] Assign fixed directional velocity (no slowing down, stopping, or steering).
- [x] Implement car movement and cleanup systems:
  - [x] Advance each car's position along its designated line: `translation += direction * speed * dt`.
  - [x] Despawn cars once their position completely exits the opposite edge of the screen ($x > W/2 + \text{margin}$ or $y < -H/2 - \text{margin}$).

## 5. Collision Detection & Player Death

- [x] Define collision query system:
  - [x] Query active `Player` and all `Car` entities each tick.
  - [x] Implement 2D Circle-to-AABB (or circle-to-oriented-box) collision detection between player radius and car bounding box.
- [x] Implement death resolution:
  - [x] Trigger death event or transition to `AppState::GameOver` upon detected overlap.
  - [x] Provide visual feedback on death (e.g., change player color to red, spawn death marker, or despawn player entity).
  - [x] Add restart trigger (e.g., press `Space` or `R` to reset player and clear traffic).

## 6. Verification & Smoke Testing

- [x] Build and launch application via `cargo run`.
- [x] Verify roads and intersection render cleanly at expected coordinates.
- [x] Test player responsiveness with all four arrow keys and boundary clamping.
- [x] Verify cars spawn smoothly off-screen, maintain constant velocity through the intersection, and despawn without entity leakage.
- [x] Test collision accuracy: confirm player survives standing clear of road lanes and triggers death upon contact with a car.
- [x] Verify reset functionality restores game state cleanly.
