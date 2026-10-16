use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::constants::*;
use crate::player::{COLOR_PLAYER, Player};
use crate::sim::{GroundPos, Heading};
use crate::spd::Spd;
use crate::state::AppState;
use crate::traffic::{Car, SpawnTimer};
/// Component representing a blood splat on the floor
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BloodSplat {
    pub seed: u64,
}

/// Event emitted when the player collides with a vehicle
#[derive(Event, Debug, Clone, Copy, PartialEq)]
pub struct PlayerDiedEvent {
    pub player_entity: Entity,
    pub car_entity: Entity,
    pub position: Vec2,
}

/// Lightweight pseudo-random generator for procedural blood splat geometry
#[derive(Debug, Clone)]
pub struct SplatRng(u64);

impl SplatRng {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x853c49e6748fea9b } else { seed })
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    pub fn next_range(&mut self, min: f32, max: f32) -> f32 {
        let t = (self.next_u32() as f32) / (u32::MAX as f32);
        min + t * (max - min)
    }
}

/// Generates a 2D mesh for a pale blood splat: an organic "circle" with procedural randomness.
pub fn create_blood_splat_mesh(seed: u64, base_radius: f32) -> Mesh {
    let mut rng = SplatRng::new(seed);
    let num_points = 24;
    let mut positions = Vec::with_capacity(num_points + 1);
    let mut normals = Vec::with_capacity(num_points + 1);
    let mut uvs = Vec::with_capacity(num_points + 1);
    let mut indices = Vec::with_capacity(num_points * 3);

    // Center vertex
    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 0.0, 1.0]);
    uvs.push([0.5, 0.5]);

    // Generate random radial perturbations around the circle
    let mut raw_offsets = Vec::with_capacity(num_points);
    for _ in 0..num_points {
        raw_offsets.push(rng.next_range(-0.35, 0.35));
    }

    // Smooth offsets cyclically so the shape forms an organic splat with lobes
    let mut smoothed_offsets = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let prev = raw_offsets[(i + num_points - 1) % num_points];
        let curr = raw_offsets[i];
        let next = raw_offsets[(i + 1) % num_points];
        smoothed_offsets.push(0.25 * prev + 0.5 * curr + 0.25 * next);
    }

    let ellipse_angle = rng.next_range(0.0, std::f32::consts::TAU);
    let ellipse_stretch = rng.next_range(0.85, 1.20);
    let max_r = base_radius * 1.6;

    for (i, &offset) in smoothed_offsets.iter().enumerate() {
        let angle = (i as f32) * std::f32::consts::TAU / (num_points as f32);
        let r = base_radius * (1.0 + offset);

        let d_angle = angle - ellipse_angle;
        let r_adj = r * (1.0 + (ellipse_stretch - 1.0) * d_angle.cos().powi(2));

        let x = r_adj * angle.cos();
        let y = r_adj * angle.sin();

        positions.push([x, y, 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([0.5 + x / (2.0 * max_r), 0.5 + y / (2.0 * max_r)]);
    }

    for i in 0..num_points {
        let next = (i + 1) % num_points;
        indices.push(0);
        indices.push(1 + i as u32);
        indices.push(1 + next as u32);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Spawns a blood splat on the floor at the specified position
pub fn spawn_blood_splat(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    position: Vec2,
    seed: u64,
) -> Entity {
    let mesh = create_blood_splat_mesh(seed, PLAYER_RADIUS * 1.15);
    let mesh_handle = meshes.add(mesh);
    let mat_handle = materials.add(ColorMaterial::from_color(COLOR_BLOOD_SPLAT));

    commands
        .spawn((
            Mesh2d(mesh_handle),
            MeshMaterial2d(mat_handle),
            Transform::from_xyz(position.x, position.y, Z_BLOOD_SPLAT),
            GroundPos(position),
            BloodSplat { seed },
        ))
        .id()
}

/// Tests 2D collision between a circle (center and radius) and an axis-aligned bounding box (center and half-extents).
///
/// Finds the closest point on the AABB to the circle center, and checks if the squared distance
/// is within the squared radius.
pub fn check_circle_aabb_collision(
    circle_center: Vec2,
    circle_radius: f32,
    box_center: Vec2,
    box_half_extents: Vec2,
) -> bool {
    let closest_point = Vec2::new(
        circle_center.x.clamp(
            box_center.x - box_half_extents.x,
            box_center.x + box_half_extents.x,
        ),
        circle_center.y.clamp(
            box_center.y - box_half_extents.y,
            box_center.y + box_half_extents.y,
        ),
    );
    let distance_sq = circle_center.distance_squared(closest_point);
    distance_sq <= circle_radius * circle_radius
}

/// Tests 2D collision between a circle (center and radius) and an oriented bounding box (center, half-extents, and rotation angle).
///
/// Transforms the circle center into the car's local coordinate frame and delegates to the AABB test.
pub fn check_circle_obb_collision(
    circle_center: Vec2,
    circle_radius: f32,
    box_center: Vec2,
    box_half_extents: Vec2,
    box_rotation: f32,
) -> bool {
    let diff = circle_center - box_center;
    let (sin_phi, cos_phi) = (-box_rotation).sin_cos();
    let local_circle_center = Vec2::new(
        diff.x * cos_phi - diff.y * sin_phi,
        diff.x * sin_phi + diff.y * cos_phi,
    );
    check_circle_aabb_collision(
        local_circle_center,
        circle_radius,
        Vec2::ZERO,
        box_half_extents,
    )
}

/// System to detect collisions between active player and any car entity.
///
/// When an overlap is detected:
/// 1. SPD counter is incremented.
/// 2. A blood splat is spawned on the floor at the collision coordinates (a pale circle with procedural randomness).
/// 3. Player is reset back to `PLAYER_SPAWN_POS` (alive, restored to `COLOR_PLAYER`).
/// 4. A `PlayerDiedEvent` is triggered via `commands.trigger(...)`.
type PlayerQueryData<'w> = (
    Entity,
    &'w mut Player,
    &'w mut GroundPos,
    Option<&'w MeshMaterial2d<ColorMaterial>>,
);

pub fn collision_detection(
    mut commands: Commands,
    mut player_query: Query<PlayerQueryData<'_>, Without<Car>>,
    car_query: Query<(Entity, &Car, &GroundPos, &Heading), Without<Player>>,
    mut materials: Option<ResMut<Assets<ColorMaterial>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut spd: Option<ResMut<Spd>>,
) {
    for (player_entity, mut player, mut player_pos, maybe_mat) in &mut player_query {
        if !player.alive {
            continue;
        }

        let p_pos = player_pos.0;

        for (car_entity, car, car_pos, car_heading) in &car_query {
            let c_pos = car_pos.0;
            let angle = car_heading.0;

            if check_circle_obb_collision(p_pos, player.radius, c_pos, car.half_extents(), angle) {
                // 1. Increment SPD
                let current_spd = if let Some(spd_res) = spd.as_deref_mut() {
                    spd_res.increment();
                    spd_res.0
                } else {
                    1
                };

                // 2. Leave a blood splat on the floor
                if let (Some(meshes_res), Some(mats_res)) =
                    (meshes.as_deref_mut(), materials.as_deref_mut())
                {
                    let seed = (p_pos.x.to_bits() as u64)
                        ^ ((p_pos.y.to_bits() as u64) << 16)
                        ^ (current_spd as u64).wrapping_mul(0x9e3779b97f4a7c15);
                    spawn_blood_splat(&mut commands, meshes_res, mats_res, p_pos, seed);
                }

                // 3. Reset player back to starting position
                player_pos.0 = PLAYER_SPAWN_POS;
                player.alive = true;

                if let Some(mut mat) =
                    maybe_mat.and_then(|h| materials.as_deref_mut()?.get_mut(&h.0))
                {
                    mat.color = COLOR_PLAYER;
                }

                // 4. Trigger PlayerDiedEvent
                commands.trigger(PlayerDiedEvent {
                    player_entity,
                    car_entity,
                    position: p_pos,
                });

                break;
            }
        }
    }
}

/// System to handle restart triggers (`Space` or `R` during `GameOver`, or `R` during `InGame`).
///
/// On restart:
/// 1. Player is marked alive and reset to `PLAYER_SPAWN_POS`.
/// 2. Player material color is restored to `COLOR_PLAYER`.
/// 3. All car entities are despawned.
/// 4. All blood splats are cleared.
/// 5. SPD counter is reset.
/// 6. The traffic `SpawnTimer` is reset.
/// 7. State transitions to `AppState::InGame`.
#[derive(SystemParam)]
pub struct RestartResources<'w, 's> {
    pub keyboard_input: Res<'w, ButtonInput<KeyCode>>,
    pub commands: Commands<'w, 's>,
    pub spawn_timer: Option<ResMut<'w, SpawnTimer>>,
    pub materials: Option<ResMut<'w, Assets<ColorMaterial>>>,
    pub spd: Option<ResMut<'w, Spd>>,
    pub next_state: Option<ResMut<'w, NextState<AppState>>>,
    pub current_state: Option<Res<'w, State<AppState>>>,
}

pub fn restart_game(
    res: RestartResources,
    mut player_query: Query<(
        &mut Player,
        &mut GroundPos,
        Option<&MeshMaterial2d<ColorMaterial>>,
    )>,
    car_query: Query<Entity, With<Car>>,
    splat_query: Query<Entity, With<BloodSplat>>,
) {
    let RestartResources {
        keyboard_input,
        mut commands,
        spawn_timer,
        mut materials,
        spd,
        next_state,
        current_state,
    } = res;
    let is_game_over = current_state
        .as_ref()
        .is_some_and(|s| *s.get() == AppState::GameOver);
    let should_restart = if is_game_over {
        keyboard_input.just_pressed(KeyCode::Space) || keyboard_input.just_pressed(KeyCode::KeyR)
    } else {
        keyboard_input.just_pressed(KeyCode::KeyR)
    };

    if should_restart {
        for (mut player, mut ground_pos, maybe_mat) in &mut player_query {
            player.alive = true;
            ground_pos.0 = PLAYER_SPAWN_POS;
            if let Some(mut mat) = maybe_mat.and_then(|h| materials.as_deref_mut()?.get_mut(&h.0)) {
                mat.color = COLOR_PLAYER;
            }
        }

        for car_entity in &car_query {
            commands.entity(car_entity).despawn();
        }

        for splat_entity in &splat_query {
            commands.entity(splat_entity).despawn();
        }

        if let Some(mut timer) = spawn_timer {
            *timer = SpawnTimer::default();
        }

        if let Some(mut spd_res) = spd {
            spd_res.reset();
        }

        if let (Some(mut next), Some(curr)) = (next_state, current_state)
            && *curr.get() != AppState::InGame
        {
            next.set(AppState::InGame);
        }
    }
}

/// Plugin configuring collision detection and death/restart systems
pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Spd>()
            .add_systems(
                Update,
                collision_detection.run_if(in_state(AppState::InGame)),
            )
            .add_systems(Update, restart_game);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::COLOR_PLAYER_DEAD;
    use bevy::input::ButtonInput;
    use bevy::state::app::StatesPlugin;

    // ==========================================
    // Circle-to-AABB Math Tests
    // ==========================================

    #[test]
    fn test_circle_inside_aabb() {
        let circle_pos = Vec2::new(0.0, 0.0);
        let radius = 10.0;
        let box_pos = Vec2::new(0.0, 0.0);
        let box_half_extents = Vec2::new(20.0, 15.0);

        assert!(check_circle_aabb_collision(
            circle_pos,
            radius,
            box_pos,
            box_half_extents
        ));
    }

    #[test]
    fn test_circle_overlapping_edge() {
        let circle_pos = Vec2::new(25.0, 0.0);
        let radius = 10.0;
        let box_pos = Vec2::new(0.0, 0.0);
        let box_half_extents = Vec2::new(20.0, 15.0);

        // Distance from box right edge (x=20) to circle center (x=25) is 5 <= 10
        assert!(check_circle_aabb_collision(
            circle_pos,
            radius,
            box_pos,
            box_half_extents
        ));
    }

    #[test]
    fn test_circle_outside_edge() {
        let circle_pos = Vec2::new(35.0, 0.0);
        let radius = 10.0;
        let box_pos = Vec2::new(0.0, 0.0);
        let box_half_extents = Vec2::new(20.0, 15.0);

        // Distance from box edge (x=20) is 15 > 10
        assert!(!check_circle_aabb_collision(
            circle_pos,
            radius,
            box_pos,
            box_half_extents
        ));
    }

    #[test]
    fn test_circle_touching_corner() {
        let box_pos = Vec2::new(0.0, 0.0);
        let box_half_extents = Vec2::new(10.0, 10.0);
        // Corner is at (10, 10). Circle at (13, 14) has distance = sqrt(3^2 + 4^2) = 5
        let circle_pos = Vec2::new(13.0, 14.0);
        let radius = 5.0;

        assert!(check_circle_aabb_collision(
            circle_pos,
            radius,
            box_pos,
            box_half_extents
        ));
    }

    #[test]
    fn test_circle_outside_corner() {
        let box_pos = Vec2::new(0.0, 0.0);
        let box_half_extents = Vec2::new(10.0, 10.0);
        // Corner is at (10, 10). Circle at (14, 15) has distance = sqrt(4^2 + 5^2) = sqrt(41) ~ 6.4 > 5
        let circle_pos = Vec2::new(14.0, 15.0);
        let radius = 5.0;

        assert!(!check_circle_aabb_collision(
            circle_pos,
            radius,
            box_pos,
            box_half_extents
        ));
    }

    #[test]
    fn test_circle_negative_coordinates() {
        let box_pos = Vec2::new(-100.0, -100.0);
        let box_half_extents = Vec2::new(24.0, 13.0);
        // Overlapping at negative coordinates
        let circle_pos = Vec2::new(-120.0, -100.0);
        let radius = 10.0;

        assert!(check_circle_aabb_collision(
            circle_pos,
            radius,
            box_pos,
            box_half_extents
        ));
    }

    #[test]
    fn test_circle_obb_collision_rotated() {
        let box_pos = Vec2::new(0.0, 0.0);
        let box_half_extents = Vec2::new(24.0, 13.0);
        let radius = 5.0;

        // Unrotated: at (35, 0), distance is 11 > 5 (no collision)
        assert!(!check_circle_obb_collision(
            Vec2::new(35.0, 0.0),
            radius,
            box_pos,
            box_half_extents,
            0.0
        ));

        // Rotated 90 degrees (FRAC_PI_2): long axis is along Y (+-24), short axis along X (+-13)
        // At (0, 27), distance from rotated top edge (y=24) is 3 <= 5 -> collision!
        assert!(check_circle_obb_collision(
            Vec2::new(0.0, 27.0),
            radius,
            box_pos,
            box_half_extents,
            std::f32::consts::FRAC_PI_2
        ));

        // At (20, 0), distance from rotated right edge (x=13) is 7 > 5 -> no collision!
        assert!(!check_circle_obb_collision(
            Vec2::new(20.0, 0.0),
            radius,
            box_pos,
            box_half_extents,
            std::f32::consts::FRAC_PI_2
        ));

        // Rotated 45 degrees (FRAC_PI_4):
        // At (23.0, -12.0), AABB check would say collision because it's inside unrotated [+-24, +-13],
        // but rotated into box local space it is at (7.78, -24.75), distance to edge (y=-13) is 11.75 > 5 -> miss!
        assert!(!check_circle_obb_collision(
            Vec2::new(23.0, -12.0),
            radius,
            box_pos,
            box_half_extents,
            std::f32::consts::FRAC_PI_4
        ));

        // Point along 45 degree heading at distance 20: (20 * cos(45), 20 * sin(45)) -> local is (20, 0) inside box -> hit!
        let angle_45 = std::f32::consts::FRAC_PI_4;
        let hit_45 = Vec2::new(20.0 * angle_45.cos(), 20.0 * angle_45.sin());
        assert!(check_circle_obb_collision(
            hit_45,
            radius,
            box_pos,
            box_half_extents,
            angle_45
        ));
    }

    // ==========================================
    // System Tests: Collision Detection & Death
    // ==========================================

    fn setup_collision_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<AppState>()
            .add_plugins(CollisionPlugin);
        app
    }

    #[test]
    fn test_no_collision_when_far_apart() {
        let mut app = setup_collision_test_app();

        // Spawn player at (0, 0)
        app.world_mut()
            .spawn((GroundPos(Vec2::ZERO), Heading::ZERO, Player::default()));

        // Spawn car far away at (200, 200)
        app.world_mut().spawn((
            GroundPos(Vec2::new(200.0, 200.0)),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));
        app.update();

        // Player should still be alive and state still InGame
        let player = app
            .world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap();
        assert!(player.alive);

        let state = app.world().resource::<State<AppState>>();
        assert_eq!(*state.get(), AppState::InGame);
    }

    #[test]
    fn test_collision_spawns_splat_increments_spd_and_resets_player() {
        let mut app = setup_collision_test_app();

        #[derive(Resource, Default)]
        struct RecordedDeaths(Vec<PlayerDiedEvent>);

        app.init_resource::<RecordedDeaths>();
        app.add_observer(
            |event: On<PlayerDiedEvent>, mut records: ResMut<RecordedDeaths>| {
                records.0.push(*event);
            },
        );

        // Set up player material
        let mut materials = app.world_mut().resource_mut::<Assets<ColorMaterial>>();
        let mat_handle = materials.add(ColorMaterial::from_color(COLOR_PLAYER));

        // Spawn player and car overlapping at (0, 0)
        let player_entity = app
            .world_mut()
            .spawn((
                GroundPos(Vec2::ZERO),
                Heading::ZERO,
                MeshMaterial2d(mat_handle.clone()),
                Player::default(),
            ))
            .id();

        let car_entity = app
            .world_mut()
            .spawn((
                GroundPos(Vec2::ZERO),
                Heading::ZERO,
                Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
            ))
            .id();

        app.update();

        // 1. SPD incremented
        let spd = app.world().resource::<Spd>();
        assert_eq!(spd.0, 1, "SPD must increment when player is run over");

        // 2. Blood splat entity spawned on the floor at (0, 0)
        let mut splat_query = app.world_mut().query::<(&BloodSplat, &GroundPos)>();
        let (splat, splat_gpos) = splat_query.single(app.world()).unwrap();
        assert_eq!(splat_gpos.0, Vec2::new(0.0, 0.0));
        let expected_seed = (0.0f32.to_bits() as u64)
            ^ ((0.0f32.to_bits() as u64) << 16)
            ^ (1u64).wrapping_mul(0x9e3779b97f4a7c15);
        assert_eq!(splat.seed, expected_seed);
        // 3. Player reset to PLAYER_SPAWN_POS and alive
        let (player, player_pos) = app
            .world_mut()
            .query::<(&Player, &GroundPos)>()
            .single(app.world())
            .unwrap();
        assert!(player.alive, "Player must remain alive after respawn");
        assert_eq!(player_pos.0, PLAYER_SPAWN_POS);

        // 4. Death event received by observer
        let recorded = app.world().resource::<RecordedDeaths>();
        assert_eq!(recorded.0.len(), 1);
        assert_eq!(recorded.0[0].player_entity, player_entity);
        assert_eq!(recorded.0[0].car_entity, car_entity);
        assert_eq!(recorded.0[0].position, Vec2::new(0.0, 0.0));

        // 5. State remains InGame
        let state = app.world().resource::<State<AppState>>();
        assert_eq!(*state.get(), AppState::InGame);
    }

    #[test]
    fn test_dead_player_does_not_trigger_additional_collisions() {
        let mut app = setup_collision_test_app();

        #[derive(Resource, Default)]
        struct DeathCounter(usize);

        app.init_resource::<DeathCounter>();
        app.add_observer(
            |_: On<PlayerDiedEvent>, mut counter: ResMut<DeathCounter>| {
                counter.0 += 1;
            },
        );

        // Spawn already dead player overlapping with a car
        let dead_player = Player {
            alive: false,
            ..Default::default()
        };

        app.world_mut()
            .spawn((GroundPos(Vec2::ZERO), Heading::ZERO, dead_player));

        app.world_mut().spawn((
            GroundPos(Vec2::ZERO),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        app.update();

        assert_eq!(app.world().resource::<DeathCounter>().0, 0);
    }

    // ==========================================
    // System Tests: Restart Trigger
    // ==========================================

    #[test]
    fn test_restart_from_game_over_with_space() {
        let mut app = setup_collision_test_app();
        app.init_resource::<SpawnTimer>();

        // Transition to GameOver
        {
            let mut next_state = app.world_mut().resource_mut::<NextState<AppState>>();
            next_state.set(AppState::GameOver);
        }
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::GameOver
        );

        let mut materials = app.world_mut().resource_mut::<Assets<ColorMaterial>>();
        let mat_handle = materials.add(ColorMaterial::from_color(COLOR_PLAYER_DEAD));

        // Spawn dead player moved from spawn pos
        let player = Player {
            alive: false,
            ..Default::default()
        };
        app.world_mut().spawn((
            GroundPos(Vec2::new(50.0, 50.0)),
            Heading::ZERO,
            MeshMaterial2d(mat_handle.clone()),
            player,
        ));

        // Spawn cars
        app.world_mut().spawn((
            GroundPos(Vec2::new(10.0, 10.0)),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Press Space
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Space);

        app.update();

        // Player should be alive, back at spawn position, and green
        let (player, ground_pos) = app
            .world_mut()
            .query::<(&Player, &GroundPos)>()
            .single(app.world())
            .unwrap();
        assert!(player.alive);
        assert_eq!(ground_pos.0, PLAYER_SPAWN_POS);

        let materials = app.world().resource::<Assets<ColorMaterial>>();
        assert_eq!(materials.get(&mat_handle).unwrap().color, COLOR_PLAYER);

        // All cars should be despawned
        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 0);

        // NextState set to InGame; transitions on next update
        let next_state = app.world().resource::<NextState<AppState>>();
        assert!(matches!(*next_state, NextState::Pending(AppState::InGame)));
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );
    }
    #[test]
    fn test_restart_from_game_over_with_r() {
        let mut app = setup_collision_test_app();
        app.init_resource::<SpawnTimer>();

        {
            let mut next_state = app.world_mut().resource_mut::<NextState<AppState>>();
            next_state.set(AppState::GameOver);
        }
        app.update();

        let player = Player {
            alive: false,
            ..Default::default()
        };
        app.world_mut()
            .spawn((GroundPos(Vec2::new(100.0, 100.0)), Heading::ZERO, player));

        app.world_mut().spawn((
            GroundPos(Vec2::ZERO),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Press R
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyR);

        app.update();

        let (player, ground_pos) = app
            .world_mut()
            .query::<(&Player, &GroundPos)>()
            .single(app.world())
            .unwrap();
        assert!(player.alive);
        assert_eq!(ground_pos.0, PLAYER_SPAWN_POS);

        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 0);
        let next_state = app.world().resource::<NextState<AppState>>();
        assert!(matches!(*next_state, NextState::Pending(AppState::InGame)));
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );
    }
    #[test]
    fn test_game_over_without_input_does_not_restart() {
        let mut app = setup_collision_test_app();
        app.init_resource::<SpawnTimer>();

        {
            let mut next_state = app.world_mut().resource_mut::<NextState<AppState>>();
            next_state.set(AppState::GameOver);
        }
        app.update();

        let player = Player {
            alive: false,
            ..Default::default()
        };
        app.world_mut()
            .spawn((GroundPos(Vec2::new(100.0, 100.0)), Heading::ZERO, player));

        app.world_mut().spawn((
            GroundPos(Vec2::ZERO),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        app.update();

        // Should remain in GameOver, player still dead, car still exists
        let player = app
            .world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap();
        assert!(!player.alive);
        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 1);
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::GameOver
        );
    }

    #[test]
    fn test_ingame_r_resets_player_and_cars() {
        let mut app = setup_collision_test_app();
        app.init_resource::<SpawnTimer>();

        app.world_mut().spawn((
            GroundPos(Vec2::new(100.0, 100.0)),
            Heading::ZERO,
            Player::default(),
        ));

        app.world_mut().spawn((
            GroundPos(Vec2::ZERO),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Press KeyR while InGame
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyR);

        app.update();

        let (player, ground_pos) = app
            .world_mut()
            .query::<(&Player, &GroundPos)>()
            .single(app.world())
            .unwrap();
        assert!(player.alive);
        assert_eq!(ground_pos.0, PLAYER_SPAWN_POS);

        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 0);
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );
    }

    #[test]
    fn test_ingame_space_does_not_restart() {
        let mut app = setup_collision_test_app();
        app.init_resource::<SpawnTimer>();

        app.world_mut().spawn((
            GroundPos(Vec2::new(100.0, 100.0)),
            Heading::ZERO,
            Player::default(),
        ));

        app.world_mut().spawn((
            GroundPos(Vec2::ZERO),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Press Space while InGame (Space is for restart only on GameOver)
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Space);

        app.update();

        let (_, ground_pos) = app
            .world_mut()
            .query::<(&Player, &GroundPos)>()
            .single(app.world())
            .unwrap();
        assert_eq!(ground_pos.0.x, 100.0);

        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 1);
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );
    }

    #[test]
    fn test_collision_accuracy_safe_corners_and_exact_contact() {
        let mut app = setup_collision_test_app();
        let geom = crate::road::RoadGeometry::default();

        // 1. Player in safe spawn corner (-160, -160)
        let player_entity = app
            .world_mut()
            .spawn((
                GroundPos(PLAYER_SPAWN_POS),
                Heading::ZERO,
                Player::default(),
            ))
            .id();

        // Spawn cars on all 4 lanes
        app.world_mut().spawn((
            GroundPos(Vec2::new(0.0, geom.east_lane_y())),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));
        app.world_mut().spawn((
            GroundPos(Vec2::new(0.0, geom.west_lane_y())),
            Heading::new(std::f32::consts::PI),
            Car::new(CAR_SPEED, -Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));
        app.world_mut().spawn((
            GroundPos(Vec2::new(geom.south_lane_x(), 0.0)),
            Heading::new(-std::f32::consts::FRAC_PI_2),
            Car::new(CAR_SPEED, -Vec2::Y, CAR_LENGTH, CAR_WIDTH),
        ));
        app.world_mut().spawn((
            GroundPos(Vec2::new(geom.north_lane_x(), 0.0)),
            Heading::new(std::f32::consts::FRAC_PI_2),
            Car::new(CAR_SPEED, Vec2::Y, CAR_LENGTH, CAR_WIDTH),
        ));

        app.update();

        let player = app.world().get::<Player>(player_entity).unwrap();
        assert!(player.alive, "Player standing clear of roads must survive");
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );

        // 2. Precision boundary check: 0.5px outside vs 0.5px inside
        let car_center = Vec2::new(0.0, 0.0);
        let car_half = Vec2::new(CAR_LENGTH / 2.0, CAR_WIDTH / 2.0); // (24.0, 13.0)
        let r = PLAYER_RADIUS; // 14.0

        // Just outside horizontal edge: distance 38.5 > 38.0
        assert!(!check_circle_aabb_collision(
            car_center + Vec2::new(car_half.x + r + 0.5, 0.0),
            r,
            car_center,
            car_half,
        ));

        // Just inside horizontal edge: distance 37.5 <= 38.0
        assert!(check_circle_aabb_collision(
            car_center + Vec2::new(car_half.x + r - 0.5, 0.0),
            r,
            car_center,
            car_half,
        ));

        // Just outside vertical edge: distance 27.5 > 27.0
        assert!(!check_circle_aabb_collision(
            car_center + Vec2::new(0.0, car_half.y + r + 0.5),
            r,
            car_center,
            car_half,
        ));

        // Just inside vertical edge: distance 26.5 <= 27.0
        assert!(check_circle_aabb_collision(
            car_center + Vec2::new(0.0, car_half.y + r - 0.5),
            r,
            car_center,
            car_half,
        ));

        // 3. Move player into direct contact with a car on Eastbound lane
        let mut player_pos = app.world_mut().get_mut::<GroundPos>(player_entity).unwrap();
        player_pos.0 = Vec2::new(0.0, geom.east_lane_y());

        app.update();

        let player = app.world().get::<Player>(player_entity).unwrap();
        assert!(player.alive, "Player must be reset alive at spawn");
        let pos = app.world().get::<GroundPos>(player_entity).unwrap();
        assert_eq!(pos.0, PLAYER_SPAWN_POS);
        let spd = app.world().resource::<Spd>();
        assert_eq!(spd.0, 1);
        let splat_count = app
            .world_mut()
            .query::<&BloodSplat>()
            .iter(app.world())
            .count();
        assert_eq!(splat_count, 1);
    }
    #[test]
    fn test_full_gameplay_death_and_clean_reset_cycle() {
        let dt = 0.1;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f32(dt),
            ))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::road::RoadGeometry>()
            .init_state::<AppState>()
            .add_plugins((
                crate::player::PlayerPlugin,
                crate::traffic::TrafficPlugin,
                CollisionPlugin,
            ));

        // Initial update: setup systems spawn player
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );

        let (player_entity, mat_handle) = {
            let mut q = app
                .world_mut()
                .query::<(Entity, &Player, &MeshMaterial2d<ColorMaterial>)>();
            let (e, p, m) = q.single(app.world()).unwrap();
            assert!(p.alive);
            (e, m.0.clone())
        };

        // 1. Move player using arrow keys
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ArrowUp);
            keys.press(KeyCode::ArrowRight);
        }
        for _ in 0..5 {
            app.update();
        }

        let player_pos = app.world().get::<GroundPos>(player_entity).unwrap().0;
        assert!(player_pos.x > PLAYER_SPAWN_POS.x);
        assert!(player_pos.y > PLAYER_SPAWN_POS.y);

        // 2. Spawn a car directly overlapping the player to cause collision
        app.world_mut().spawn((
            GroundPos(player_pos),
            Heading::ZERO,
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Update to run collision detection
        app.update();

        // Player gets run over:
        // - Blood splat left on floor
        let splat_count = app
            .world_mut()
            .query::<&BloodSplat>()
            .iter(app.world())
            .count();
        assert_eq!(
            splat_count, 1,
            "Blood splat entity must be spawned on death"
        );

        // - SPD incremented
        assert_eq!(
            app.world().resource::<Spd>().0,
            1,
            "SPD counter must increment"
        );

        // - Player reset back to starting position, alive and ready to move
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert!(player.alive, "Player must be alive at spawn");
        let player_gpos = app.world().get::<GroundPos>(player_entity).unwrap();
        assert_eq!(player_gpos.0, PLAYER_SPAWN_POS);
        assert_eq!(
            app.world()
                .resource::<Assets<ColorMaterial>>()
                .get(&mat_handle)
                .unwrap()
                .color,
            COLOR_PLAYER
        );
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );

        // 3. Arrow keys immediately move player from spawn without needing any manual restart
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ArrowUp);
        }
        for _ in 0..3 {
            app.update();
        }
        let resumed_pos = app.world().get::<GroundPos>(player_entity).unwrap().0;
        assert!(
            resumed_pos.y > PLAYER_SPAWN_POS.y,
            "Player should move freely after death reset"
        );

        // 4. Test manual restart via R key resets SPD and clears splats and cars
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyR);
        }
        app.update();

        assert_eq!(
            app.world().resource::<Spd>().0,
            0,
            "SPD must reset on R key"
        );
        let remaining_splats = app
            .world_mut()
            .query::<&BloodSplat>()
            .iter(app.world())
            .count();
        assert_eq!(
            remaining_splats, 0,
            "Blood splats should be cleared on reset"
        );
        let remaining_cars = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(remaining_cars, 0, "Cars should be despawned on reset");
    }

    #[test]
    fn test_blood_splat_mesh_and_randomness() {
        let mesh1 = create_blood_splat_mesh(12345, 14.0);
        let mesh2 = create_blood_splat_mesh(67890, 14.0);

        let pos1 = mesh1
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        let pos2 = mesh2
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();

        assert_eq!(pos1.len(), 25); // 1 center + 24 perimeter
        assert_eq!(pos2.len(), 25);

        // Verify center vertex is (0, 0, 0)
        assert_eq!(pos1[0], [0.0, 0.0, 0.0]);
        assert_eq!(pos2[0], [0.0, 0.0, 0.0]);

        // Verify that different seeds produce different vertex positions (randomness in shape)
        let mut has_difference = false;
        for i in 1..25 {
            if (pos1[i][0] - pos2[i][0]).abs() > 0.01 || (pos1[i][1] - pos2[i][1]).abs() > 0.01 {
                has_difference = true;
                break;
            }
        }
        assert!(
            has_difference,
            "Different seeds must produce different splat shapes"
        );

        // Verify indices
        let indices = mesh1.indices().unwrap();
        assert_eq!(indices.len(), 24 * 3);
    }

    #[test]
    fn test_collision_on_curved_road() {
        let mut app = setup_collision_test_app();

        // Place car along the curve at 45 degrees
        let angle = std::f32::consts::FRAC_PI_4;
        let r = CURVE_RADIUS + LANE_WIDTH / 2.0;
        let curve_car_pos = Vec2::new(
            CURVE_START_X + r * angle.sin(),
            CURVE_CENTER.y - r * angle.cos(),
        );

        let _car_entity = app
            .world_mut()
            .spawn((
                GroundPos(curve_car_pos),
                Heading::new(angle),
                Car::new(
                    CAR_SPEED,
                    Vec2::new(angle.cos(), angle.sin()),
                    CAR_LENGTH,
                    CAR_WIDTH,
                ),
            ))
            .id();

        // Place player directly on the curve at the car position
        let player_entity = app
            .world_mut()
            .spawn((Player::default(), GroundPos(curve_car_pos), Heading::ZERO))
            .id();

        app.update();

        // Collision should be detected: player reset to spawn, SPD incremented, splat spawned
        let p_pos = app.world().get::<GroundPos>(player_entity).unwrap();
        assert_eq!(p_pos.0, PLAYER_SPAWN_POS);
        let spd = app.world().resource::<Spd>();
        assert_eq!(spd.0, 1);

        let mut splat_query = app.world_mut().query::<(&BloodSplat, &GroundPos)>();
        let splats: Vec<(&BloodSplat, &GroundPos)> = splat_query.iter(app.world()).collect();
        assert_eq!(splats.len(), 1);
        assert_eq!(splats[0].1.0, curve_car_pos);
    }
}
