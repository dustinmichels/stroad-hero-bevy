# Stroad Hero: Masterplan

This document charts the roadmap from the initial 2D intersection prototype (`plan.md`) to the full activist satire game envisioned in `README.md`.

## 1. Vision & Core Goals (from `README.md`)

- **Core Narrative & Premise**: The player controls **Mats**, a young Dutch boy living in Anywhere, USA. His mother has asked him to do some errands. He is trying to do so without going splat.
- **Core Mechanics**: A Frogger-like element where the player must dodge high-speed cars.
- **Satirical Environment**: Highlights the misery of "stroads" (sprawling, high-speed streets) and sprawling urban landscape, including parody stores like _Walgroins_ and _Taco Hell_.
- **Visual Target**: Open-ended — picturing pixel art graphics, but not married to the idea.
- **Simulation Model**: Suggests the underlying model could be 3D to simulate real traffic patterns (cars drive to destinations, park, driver gets out, visits store, gets back in, drives elsewhere).

### 1.1 Proposed & Extended Ideas (Community / Brainstorm)

- Mats contrasting suburban stroad reality against bike/pedestrian infrastructure.
- Specific infrastructure hazards: center continuous left-turn lanes ("suicide lanes"), disconnected sidewalks ending in dirt, delayed pedestrian "beg buttons", channelized slip lanes, and parking lagoons.
- Additional parody brands (_Depot Depot_, _Sub-Optimal_, _Big Gas_) and monolithic strip-mall pylon signs.

## 2. Current State vs. Target State

| Dimension       | Current MVP (`plan.md`)                      | Target Game (`masterplan.md`)                                               |
| :-------------- | :------------------------------------------- | :-------------------------------------------------------------------------- |
| **World**       | Single 800×800 static cross-intersection     | Scrolling or multi-screen commercial stroad corridor                        |
| **Roadway**     | 2-lane perpendicular roads (1 lane each way) | 4–6 lane arterial with center turn lane, slip lanes, curb cuts              |
| **Player**      | Green circle primitive, 4-way arrow keys     | Mats sprite, walking animations, Dutch bike or pedestrian options           |
| **Traffic**     | Rectangular primitives moving straight       | Diverse vehicle types (oversized SUVs, trucks, sedans) with turning/parking |
| **Objectives**  | Infinite survival (dodge until hit, reset)   | Errand checklist (visit stores, pick up items, return home alive)           |
| **Environment** | Abstract road lines and asphalt rectangles   | Parody storefronts, asphalt parking lots, missing sidewalks, beg buttons    |
| **Platform**    | Native desktop only                          | Native desktop + WebAssembly in browser via Trunk                           |

---

## 3. Strategic Architectural Decisions

### 3.1 Simulation Model vs. Visual Presentation (2D, 3D, and Hybrid)

- **README Guidance**: Notes that the underlying model should perhaps be 3D to simulate real traffic patterns (destination routing, parking lots, drivers getting in and out), while visuals might be pixel art.
- **Decoupled Architecture (Recommended Path)**:
  1. **Logical Simulation (2D Ground Plane)**: Real-world vehicle navigation, lane graphs, parking stalls, and pedestrian errand routes exist on a 2D planar coordinate space ($X, Y$). Real traffic simulation does not require vertical 3D physics unless simulating multi-level parking structures or complex elevation overpasses.
  2. **Current Codebase vs. 3D Decoupling Prerequisite**:
     - Currently, game logic reads and writes Bevy's `Transform.translation.xy` directly (in `car_movement`, `car_cleanup`, `player_movement`, and `check_collisions`), using $Z$ as the 2D rendering layer.
     - Because Bevy 3D uses a right-handed coordinate system (+Y up, +X right, -Z forward/north), ground positions in 3D reside on the $XZ$ plane. 2D $(x, y)$ coordinates map to 3D $(x, 0, -y)$. Directly swapping to a 3D camera without decoupling would break all existing movement and collision queries (or mirror the world and flip traffic sides if mapped to $+z$).
     - **Required Refactor**: Introduce a dedicated simulation component (e.g., `GroundPos(pub Vec2)` and `Heading(pub f32)`) as the single source of truth for movement, pathfinding, and collision. A sync system then writes to `Transform`:
       - _2D Sync_: `transform.translation = Vec3::new(pos.x, pos.y, z_layer)`
       - _3D Sync_: `transform.translation = Vec3::new(pos.x, 0.0, -pos.y); transform.rotation = Quat::from_rotation_y(heading);`
     - Performing this refactor early (Phase 1) ensures subsequent waypoint graphs, parking AI, and store triggers remain completely agnostic to whether visuals are 2D or 3D.
  3. **Visual Presentation Options**:
     - _Immediate (2D Sprites / Pixel Art)_: Fast iteration with `Camera2d` and `Sprite`, zero 3D modeling overhead.
     - _Future Expansion (3D Presentation)_: Switch camera to `Camera3d` (isometric or perspective) and bind low-poly meshes or billboarded 2.5D sprites, leaving simulation logic 100% untouched.
     - _Chosen Direction_: isometric pixel art (`pics/ideas/idea.jpg`, refining `pics/ideas/01_isometric_pixel_art.png`). Asset pipeline, camera setup, and how the view binds to the sim are in `docs/isometric-view.md`.
- **Web Budget**: Target is < 15 MB uncompressed wasm (ideally < 10 MB per `docs/web-plan.md`). A 2D simulation layer easily accommodates either visual target while staying within web constraints.

---

## 4. Development Phases & Milestones

### Phase 1: Simulation Decoupling & Errand Game Loop

Transform the tech demo into an objective-driven game with win/loss states, decoupling position logic for future 3D visual support.

- [ ] **Prerequisite: Simulation Position Decoupling**:
  - [ ] Introduce `GroundPos(pub Vec2)` and `Heading(pub f32)` components for player and vehicles.
  - [ ] Refactor `player_movement`, `car_movement`, `car_cleanup`, and `check_collisions` to operate on `GroundPos` instead of `Transform`.
  - [ ] Add `sync_transforms_2d` system to project `GroundPos` into `Transform.translation.xy` at proper $Z$ layers.
- [ ] **Errand Data Structures**:
  - [ ] Define `Errand` component and `ErrandList` resource (e.g., Target Store, Item Required, Completed status).
  - [ ] Implement initial scenario: Mother's Checklist (e.g. 1. Pick up prescription at _Walgroins_, 2. Buy family dinner at _Taco Hell_, 3. Return home).
- [ ] **Store Entities & Interactivity**:
  - [ ] Create `Store` component with `StoreType` enum and AABB interaction trigger zones.
  - [ ] Add interaction detection when Mats enters a store zone (auto-interact or action prompt).
  - [ ] Errand completion notification and checklist update.
- [ ] **HUD & Game State Progression**:
  - [ ] In-game UI overlay displaying active errand checklist and status.
  - [ ] Victory state (`AppState::Victory`) when all errands are completed and Mats returns home.
  - [ ] Score / stats screen on game over or victory (e.g., near misses, time spent waiting at beg buttons, miles walked across asphalt).

### Phase 2: Authentic Stroad Geography & Level Layout

Replace the generic 2-lane intersection with genuine stroad morphology.

- [ ] **Multi-Lane Arterial Roadway**:
  - [ ] 4 to 6 continuous travel lanes with realistic high-speed traffic flow.
  - [ ] Continuous two-way left-turn lane (the "suicide lane") down the center.
  - [ ] Channelized right-turn slip lanes with non-yielding vehicle behavior.
- [ ] **Pedestrian Hostility Elements**:
  - [ ] Discontinuous sidewalks (paved paths that suddenly terminate into dirt, weeds, or drainage ditches).
  - [ ] "Beg Buttons" at signalized crossings: long pedestrian cycle delays (e.g., 45-second wait for a 6-second crossing window).
  - [ ] Unsignalized mid-block crossings with multi-lane threat (cars in lane 1 stop, blocking view of speeding car in lane 2).
- [ ] **Commercial Strip Mall & Parking Sea**:
  - [ ] Expansive parking lots separating storefront entrances from the roadway by hundreds of virtual feet.
  - [ ] Curb cuts: multiple wide driveway entrances intersecting the sidewalk/shoulder.

### Phase 3: Traffic & Driver Behavior Simulation

Evolve simple straight-line car spawning into a dynamic vehicular environment.

- [ ] **Vehicle Variety & Physical Dimensions**:
  - [ ] Add distinct vehicle archetypes: Compact Sedan, Heavy Duty Pickup Truck, Full-Size SUV, Commercial Delivery Van.
  - [ ] Vary acceleration, top speed, and collision footprints (oversized front blind zones).
- [ ] **Waypoint & Navigation Graph**:
  - [ ] Waypoint node system mapping arterial lanes, turn pockets, curb cut driveways, and parking lot lanes.
  - [ ] Vehicle state machine: `Cruising`, `DeceleratingToTurn`, `EnteringLot`, `SearchingForSpot`, `Parked`, `Reversing`, `ExitingLot`.
- [ ] **Parking Lot Dynamics**:
  - [ ] Marked parking stalls throughout strip mall lots.
  - [ ] Vehicles randomly selecting stalls, parking for a duration, and reversing out into parking aisles (hazard for Mats).

### Phase 4: Visual Satire & Pixel Art Pipeline

Replace color primitives with cohesive 2D pixel art and satirical environmental storytelling.

- [ ] **Mats Character Sprite**:
  - [ ] Pixel art sprite sheet for Mats (distinctive bright rain jacket or Dutch attire).
  - [ ] 4-directional walking animations and death animation / marker.
- [ ] **Vehicle Sprites**:
  - [ ] Pixel art sprites for sedans, oversized trucks, SUVs, and delivery vans with directional rotations.
  - [ ] Turn signal indicators (or lack thereof, reflecting erratic driving).
- [ ] **Parody Storefronts & Strip Mall Architecture**:
  - [ ] Parody brand facades: _Walgroins_, _Taco Hell_, _Depot Depot_, _Sub-Optimal_, _Big Gas_.
  - [ ] Monolithic road pylon signs advertising fast food and retail combos.
  - [ ] Environmental props: shopping carts abandoned in bushes, chain-link fences, utility poles with messy wires.

### Phase 5: Audio & Satirical Atmosphere

Build auditory feedback reinforcing the stress and absurdity of stroad design.

- [ ] **Ambient & Traffic Audio**:
  - [ ] Relentless tire roar, engine acceleration, and wind rush.
  - [ ] Sudden horn honks when Mats crosses outside crosswalks or delays traffic.
  - [ ] Chirping / beeping accessible pedestrian signal tones for crosswalks.
- [ ] **Narrative & UI Audio**:
  - [ ] Dialogue soundbites or text prompts from Mats's mother with Dutch accent / phrasing.
  - [ ] Satirical jingles upon entering parody store locations.

### Phase 6: Web Build & Distribution (`docs/web-plan.md`)

Execute the steps in `docs/web-plan.md` to ensure immediate browser playability.

- [ ] Complete toolchain setup (`trunk`, `wasm32-unknown-unknown`, `wasm-opt`).
- [ ] Configure `index.html` canvas binding and responsive parent container.
- [ ] Implement browser audio unlock (click-to-start title screen).
- [ ] Audit and enforce wasm size budget (< 15 MB uncompressed, target < 10 MB).
- [ ] Setup GitHub Actions CI pipeline deploying to GitHub Pages and itch.io.
