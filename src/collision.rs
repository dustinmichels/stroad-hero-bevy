use bevy::prelude::*;

use crate::constants::*;
use crate::player::{Player, COLOR_PLAYER, COLOR_PLAYER_DEAD};
use crate::state::AppState;
use crate::traffic::{Car, SpawnTimer};

/// Event emitted when the player collides with a vehicle
#[derive(Event, Debug, Clone, Copy, PartialEq)]
pub struct PlayerDiedEvent {
    pub player_entity: Entity,
    pub car_entity: Entity,
    pub position: Vec2,
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
        circle_center
            .x
            .clamp(box_center.x - box_half_extents.x, box_center.x + box_half_extents.x),
        circle_center
            .y
            .clamp(box_center.y - box_half_extents.y, box_center.y + box_half_extents.y),
    );
    let distance_sq = circle_center.distance_squared(closest_point);
    distance_sq <= circle_radius * circle_radius
}

/// System to detect collisions between active player and any car entity.
///
/// When an overlap is detected:
/// 1. Player is marked as not alive (`player.alive = false`).
/// 2. Player material color changes to `COLOR_PLAYER_DEAD`.
/// 3. A `PlayerDiedEvent` is triggered via `commands.trigger(...)`.
/// 4. State transitions to `AppState::GameOver`.
pub fn collision_detection(
    mut commands: Commands,
    mut player_query: Query<(
        Entity,
        &mut Player,
        &Transform,
        Option<&MeshMaterial2d<ColorMaterial>>,
    )>,
    car_query: Query<(Entity, &Car, &Transform)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (player_entity, mut player, player_transform, maybe_mat) in &mut player_query {
        if !player.alive {
            continue;
        }

        let player_pos = player_transform.translation.truncate();

        for (car_entity, car, car_transform) in &car_query {
            let car_pos = car_transform.translation.truncate();

            if check_circle_aabb_collision(player_pos, player.radius, car_pos, car.half_extents()) {
                player.alive = false;

                if let Some(mat_handle) = maybe_mat {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.color = COLOR_PLAYER_DEAD;
                    }
                }

                commands.trigger(PlayerDiedEvent {
                    player_entity,
                    car_entity,
                    position: player_pos,
                });

                next_state.set(AppState::GameOver);
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
/// 4. The traffic `SpawnTimer` is reset.
/// 5. State transitions to `AppState::InGame`.
pub fn restart_game(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut player_query: Query<(
        &mut Player,
        &mut Transform,
        Option<&MeshMaterial2d<ColorMaterial>>,
    )>,
    car_query: Query<Entity, With<Car>>,
    spawn_timer: Option<ResMut<SpawnTimer>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut next_state: ResMut<NextState<AppState>>,
    current_state: Res<State<AppState>>,
) {
    let should_restart = match *current_state.get() {
        AppState::GameOver => {
            keyboard_input.just_pressed(KeyCode::Space) || keyboard_input.just_pressed(KeyCode::KeyR)
        }
        AppState::InGame => keyboard_input.just_pressed(KeyCode::KeyR),
    };

    if should_restart {
        for (mut player, mut transform, maybe_mat) in &mut player_query {
            player.alive = true;
            transform.translation = Vec3::new(PLAYER_SPAWN_POS.x, PLAYER_SPAWN_POS.y, Z_PLAYER);

            if let Some(mat_handle) = maybe_mat {
                if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                    mat.color = COLOR_PLAYER;
                }
            }
        }

        for car_entity in &car_query {
            commands.entity(car_entity).despawn();
        }

        if let Some(mut timer) = spawn_timer {
            *timer = SpawnTimer::default();
        }

        if *current_state.get() != AppState::InGame {
            next_state.set(AppState::InGame);
        }
    }
}

/// Plugin configuring collision detection and death/restart systems
pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonInput<KeyCode>>()
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
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, Z_PLAYER),
            Player::default(),
        ));

        // Spawn car far away at (200, 200)
        app.world_mut().spawn((
            Transform::from_xyz(200.0, 200.0, Z_CAR),
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
    fn test_collision_triggers_death_event_and_state_change() {
        let mut app = setup_collision_test_app();

        #[derive(Resource, Default)]
        struct RecordedDeaths(Vec<PlayerDiedEvent>);

        app.init_resource::<RecordedDeaths>();
        app.add_observer(|event: On<PlayerDiedEvent>, mut records: ResMut<RecordedDeaths>| {
            records.0.push(*event);
        });

        // Set up player material
        let mut materials = app.world_mut().resource_mut::<Assets<ColorMaterial>>();
        let mat_handle = materials.add(ColorMaterial::from_color(COLOR_PLAYER));

        // Spawn player and car overlapping at (0, 0)
        let player_entity = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.0, Z_PLAYER),
                MeshMaterial2d(mat_handle.clone()),
                Player::default(),
            ))
            .id();

        let car_entity = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.0, Z_CAR),
                Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
            ))
            .id();

        app.update();

        // 1. Player marked as not alive
        let player = app
            .world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap();
        assert!(!player.alive);

        // 2. NextState set to GameOver; transitions on next update
        let next_state = app.world().resource::<NextState<AppState>>();
        assert!(matches!(*next_state, NextState::Pending(AppState::GameOver)));
        app.update();
        let state = app.world().resource::<State<AppState>>();
        assert_eq!(*state.get(), AppState::GameOver);
        // 3. Death event received by observer
        let recorded = app.world().resource::<RecordedDeaths>();
        assert_eq!(recorded.0.len(), 1);
        assert_eq!(recorded.0[0].player_entity, player_entity);
        assert_eq!(recorded.0[0].car_entity, car_entity);
        assert_eq!(recorded.0[0].position, Vec2::new(0.0, 0.0));
        // 4. Player material changed to COLOR_PLAYER_DEAD
        let materials = app.world().resource::<Assets<ColorMaterial>>();
        let mat = materials.get(&mat_handle).unwrap();
        assert_eq!(mat.color, COLOR_PLAYER_DEAD);
    }

    #[test]
    fn test_dead_player_does_not_trigger_additional_collisions() {
        let mut app = setup_collision_test_app();

        #[derive(Resource, Default)]
        struct DeathCounter(usize);

        app.init_resource::<DeathCounter>();
        app.add_observer(|_: On<PlayerDiedEvent>, mut counter: ResMut<DeathCounter>| {
            counter.0 += 1;
        });

        // Spawn already dead player overlapping with a car
        let mut dead_player = Player::default();
        dead_player.alive = false;

        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, Z_PLAYER),
            dead_player,
        ));

        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, Z_CAR),
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
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::GameOver);

        let mut materials = app.world_mut().resource_mut::<Assets<ColorMaterial>>();
        let mat_handle = materials.add(ColorMaterial::from_color(COLOR_PLAYER_DEAD));

        // Spawn dead player moved from spawn pos
        let mut player = Player::default();
        player.alive = false;
        app.world_mut().spawn((
            Transform::from_xyz(50.0, 50.0, Z_PLAYER),
            MeshMaterial2d(mat_handle.clone()),
            player,
        ));

        // Spawn cars
        app.world_mut().spawn((
            Transform::from_xyz(10.0, 10.0, Z_CAR),
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Press Space
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Space);

        app.update();

        // Player should be alive, back at spawn position, and green
        let (player, transform) = app
            .world_mut()
            .query::<(&Player, &Transform)>()
            .single(app.world())
            .unwrap();
        assert!(player.alive);
        assert_eq!(transform.translation, Vec3::new(PLAYER_SPAWN_POS.x, PLAYER_SPAWN_POS.y, Z_PLAYER));

        let materials = app.world().resource::<Assets<ColorMaterial>>();
        assert_eq!(materials.get(&mat_handle).unwrap().color, COLOR_PLAYER);

        // All cars should be despawned
        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 0);

        // NextState set to InGame; transitions on next update
        let next_state = app.world().resource::<NextState<AppState>>();
        assert!(matches!(*next_state, NextState::Pending(AppState::InGame)));
        app.update();
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);
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

        let mut player = Player::default();
        player.alive = false;
        app.world_mut().spawn((
            Transform::from_xyz(100.0, 100.0, Z_PLAYER),
            player,
        ));

        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, Z_CAR),
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Press R
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyR);

        app.update();

        let (player, transform) = app
            .world_mut()
            .query::<(&Player, &Transform)>()
            .single(app.world())
            .unwrap();
        assert!(player.alive);
        assert_eq!(transform.translation, Vec3::new(PLAYER_SPAWN_POS.x, PLAYER_SPAWN_POS.y, Z_PLAYER));

        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 0);
        let next_state = app.world().resource::<NextState<AppState>>();
        assert!(matches!(*next_state, NextState::Pending(AppState::InGame)));
        app.update();
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);
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

        let mut player = Player::default();
        player.alive = false;
        app.world_mut().spawn((
            Transform::from_xyz(100.0, 100.0, Z_PLAYER),
            player,
        ));

        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, Z_CAR),
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        app.update();

        // Should remain in GameOver, player still dead, car still exists
        let player = app.world_mut().query::<&Player>().single(app.world()).unwrap();
        assert!(!player.alive);
        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 1);
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::GameOver);
    }

    #[test]
    fn test_ingame_r_resets_player_and_cars() {
        let mut app = setup_collision_test_app();
        app.init_resource::<SpawnTimer>();

        app.world_mut().spawn((
            Transform::from_xyz(100.0, 100.0, Z_PLAYER),
            Player::default(),
        ));

        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, Z_CAR),
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Press KeyR while InGame
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::KeyR);

        app.update();

        let (player, transform) = app
            .world_mut()
            .query::<(&Player, &Transform)>()
            .single(app.world())
            .unwrap();
        assert!(player.alive);
        assert_eq!(transform.translation, Vec3::new(PLAYER_SPAWN_POS.x, PLAYER_SPAWN_POS.y, Z_PLAYER));

        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 0);
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);
    }

    #[test]
    fn test_ingame_space_does_not_restart() {
        let mut app = setup_collision_test_app();
        app.init_resource::<SpawnTimer>();

        app.world_mut().spawn((
            Transform::from_xyz(100.0, 100.0, Z_PLAYER),
            Player::default(),
        ));

        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, Z_CAR),
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Press Space while InGame (Space is for restart only on GameOver)
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Space);

        app.update();

        let (_, transform) = app
            .world_mut()
            .query::<(&Player, &Transform)>()
            .single(app.world())
            .unwrap();
        assert_eq!(transform.translation.x, 100.0);

        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 1);
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);
    }

    #[test]
    fn test_collision_accuracy_safe_corners_and_exact_contact() {
        let mut app = setup_collision_test_app();
        let geom = crate::road::RoadGeometry::default();

        // 1. Player in safe spawn corner (-160, -160)
        let player_entity = app
            .world_mut()
            .spawn((
                Transform::from_xyz(PLAYER_SPAWN_POS.x, PLAYER_SPAWN_POS.y, Z_PLAYER),
                Player::default(),
            ))
            .id();

        // Spawn cars on all 4 lanes
        app.world_mut().spawn((
            Transform::from_xyz(0.0, geom.east_lane_y(), Z_CAR),
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));
        app.world_mut().spawn((
            Transform::from_xyz(0.0, geom.west_lane_y(), Z_CAR),
            Car::new(CAR_SPEED, -Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));
        app.world_mut().spawn((
            Transform::from_xyz(geom.south_lane_x(), 0.0, Z_CAR),
            Car::new(CAR_SPEED, -Vec2::Y, CAR_WIDTH, CAR_LENGTH),
        ));
        app.world_mut().spawn((
            Transform::from_xyz(geom.north_lane_x(), 0.0, Z_CAR),
            Car::new(CAR_SPEED, Vec2::Y, CAR_WIDTH, CAR_LENGTH),
        ));

        app.update();

        let player = app.world().get::<Player>(player_entity).unwrap();
        assert!(player.alive, "Player standing clear of roads must survive");
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);

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
        let mut player_trans = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        player_trans.translation = Vec3::new(0.0, geom.east_lane_y(), Z_PLAYER);

        app.update();

        let player = app.world().get::<Player>(player_entity).unwrap();
        assert!(!player.alive, "Player in car path must trigger death");
        let next_state = app.world().resource::<NextState<AppState>>();
        assert!(matches!(*next_state, NextState::Pending(AppState::GameOver)));
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
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);

        let (player_entity, mat_handle) = {
            let mut q = app.world_mut().query::<(Entity, &Player, &MeshMaterial2d<ColorMaterial>)>();
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

        let player_pos = app.world().get::<Transform>(player_entity).unwrap().translation.truncate();
        assert!(player_pos.x > PLAYER_SPAWN_POS.x);
        assert!(player_pos.y > PLAYER_SPAWN_POS.y);

        // 2. Spawn a car directly overlapping the player to cause collision
        app.world_mut().spawn((
            Transform::from_xyz(player_pos.x, player_pos.y, Z_CAR),
            Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
        ));

        // Update to run collision detection
        app.update();

        // Player must be dead, color red, state transitioning to GameOver
        assert!(!app.world().get::<Player>(player_entity).unwrap().alive);
        assert_eq!(
            app.world().resource::<Assets<ColorMaterial>>().get(&mat_handle).unwrap().color,
            COLOR_PLAYER_DEAD
        );

        // Update to complete state transition to GameOver
        app.update();
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::GameOver);

        // While dead in GameOver, arrow key inputs must NOT move the player
        let pos_before_move_attempt = app.world().get::<Transform>(player_entity).unwrap().translation;
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ArrowUp);
        }
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world().get::<Transform>(player_entity).unwrap().translation,
            pos_before_move_attempt,
            "Dead player must not move"
        );

        // 3. Trigger Reset via Space
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::Space);
        }
        app.update();

        // Verify clean reset state:
        // - Player alive
        let player = app.world().get::<Player>(player_entity).unwrap();
        assert!(player.alive, "Player must be revived on reset");
        // - Position reset to PLAYER_SPAWN_POS
        let player_trans = app.world().get::<Transform>(player_entity).unwrap();
        assert_eq!(
            player_trans.translation,
            Vec3::new(PLAYER_SPAWN_POS.x, PLAYER_SPAWN_POS.y, Z_PLAYER)
        );
        // - Color reset to COLOR_PLAYER
        assert_eq!(
            app.world().resource::<Assets<ColorMaterial>>().get(&mat_handle).unwrap().color,
            COLOR_PLAYER
        );
        // - All cars cleared
        let car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(car_count, 0, "All cars must be despawned on reset");
        // - Next state is InGame
        app.update();
        assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);

        // 4. Verify gameplay immediately resumes
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ArrowUp);
        }
        for _ in 0..3 {
            app.update();
        }

        let resumed_pos = app.world().get::<Transform>(player_entity).unwrap().translation;
        assert!(
            resumed_pos.y > PLAYER_SPAWN_POS.y,
            "Player should move freely after reset"
        );
    }
}
