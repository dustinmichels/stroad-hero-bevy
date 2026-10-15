use bevy::prelude::*;

use crate::constants::*;
use crate::state::AppState;

/// Distinct bright color for the player circle
pub const COLOR_PLAYER: Color = Color::srgb(0.2, 0.85, 0.35);

/// Color for the player circle upon death
pub const COLOR_PLAYER_DEAD: Color = Color::srgb(0.9, 0.2, 0.2);

/// Component representing the player character
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Player {
    pub radius: f32,
    pub speed: f32,
    pub alive: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            radius: PLAYER_RADIUS,
            speed: PLAYER_SPEED,
            alive: true,
        }
    }
}

impl Player {
    pub fn new(radius: f32, speed: f32) -> Self {
        Self {
            radius,
            speed,
            alive: true,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }
}

/// Clamp coordinates within virtual world boundaries taking player radius into account
pub fn clamp_position(pos: Vec2, radius: f32) -> Vec2 {
    let min_x = WORLD_BOUNDS_MIN.x + radius;
    let max_x = WORLD_BOUNDS_MAX.x - radius;
    let min_y = WORLD_BOUNDS_MIN.y + radius;
    let max_y = WORLD_BOUNDS_MAX.y - radius;

    Vec2::new(pos.x.clamp(min_x, max_x), pos.y.clamp(min_y, max_y))
}

/// System to spawn the player character as a circle mesh
pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let circle = meshes.add(Circle::new(PLAYER_RADIUS));
    let material = materials.add(ColorMaterial::from_color(COLOR_PLAYER));

    commands.spawn((
        Mesh2d(circle),
        MeshMaterial2d(material),
        Transform::from_xyz(PLAYER_SPAWN_POS.x, PLAYER_SPAWN_POS.y, Z_PLAYER),
        Player::default(),
    ));
}

/// System to process arrow key (and WASD) input and move the player within bounds
pub fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&Player, &mut Transform)>,
) {
    let mut direction = Vec2::ZERO;

    if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    let move_direction = direction.normalize_or_zero();

    for (player, mut transform) in &mut query {
        if !player.alive {
            continue;
        }

        let delta = move_direction * player.speed * time.delta_secs();
        let new_pos = transform.translation.truncate() + delta;
        let clamped = clamp_position(new_pos, player.radius);

        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}

/// Plugin configuring the player systems
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Startup, setup_player)
            .add_systems(Update, player_movement.run_if(in_state(AppState::InGame)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::ButtonInput;
    use bevy::state::app::StatesPlugin;

    #[test]
    fn test_player_defaults() {
        let player = Player::default();
        assert_eq!(player.radius, PLAYER_RADIUS);
        assert_eq!(player.speed, PLAYER_SPEED);
        assert!(player.is_alive());
        assert!(player.alive);
    }

    #[test]
    fn test_clamp_position_inside_bounds() {
        let inside = Vec2::new(0.0, 50.0);
        let clamped = clamp_position(inside, PLAYER_RADIUS);
        assert_eq!(clamped, inside);
    }

    #[test]
    fn test_clamp_position_outside_bounds() {
        let radius = 10.0;
        // Min bounds are -400.0, max are 400.0. With radius 10, valid range is [-390.0, 390.0]
        let too_far_left = Vec2::new(-500.0, 0.0);
        let clamped_left = clamp_position(too_far_left, radius);
        assert_eq!(clamped_left.x, -390.0);
        assert_eq!(clamped_left.y, 0.0);

        let too_far_right = Vec2::new(500.0, 0.0);
        let clamped_right = clamp_position(too_far_right, radius);
        assert_eq!(clamped_right.x, 390.0);

        let too_far_up = Vec2::new(0.0, 450.0);
        let clamped_up = clamp_position(too_far_up, radius);
        assert_eq!(clamped_up.y, 390.0);

        let too_far_down = Vec2::new(0.0, -450.0);
        let clamped_down = clamp_position(too_far_down, radius);
        assert_eq!(clamped_down.y, -390.0);
    }

    #[test]
    fn test_player_spawn_is_safe_from_roads() {
        // Road width is 120.0, so road extends from -60.0 to +60.0 on both axes.
        let half_road = ROAD_WIDTH / 2.0;
        let player_box_min = PLAYER_SPAWN_POS - Vec2::splat(PLAYER_RADIUS);
        let player_box_max = PLAYER_SPAWN_POS + Vec2::splat(PLAYER_RADIUS);

        // Verify player is strictly outside the horizontal road [-half_w..half_w, -60..60]
        // and vertical road [-60..60, -half_h..half_h]
        let overlaps_horiz_road = player_box_max.y >= -half_road && player_box_min.y <= half_road;
        let overlaps_vert_road = player_box_max.x >= -half_road && player_box_min.x <= half_road;

        assert!(
            !overlaps_horiz_road,
            "Player spawn should not overlap horizontal road"
        );
        assert!(
            !overlaps_vert_road,
            "Player spawn should not overlap vertical road"
        );
    }

    #[test]
    fn test_setup_player_spawns_entity() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_state::<AppState>()
            .add_systems(Startup, setup_player);

        app.update();

        let mut query =
            app.world_mut()
                .query::<(&Player, &Transform, &Mesh2d, &MeshMaterial2d<ColorMaterial>)>();
        let players: Vec<(&Player, &Transform, &Mesh2d, &MeshMaterial2d<ColorMaterial>)> =
            query.iter(app.world()).collect();

        assert_eq!(players.len(), 1, "Exactly one player should be spawned");
        let (player, transform, _mesh, _mat) = players[0];
        assert_eq!(player.radius, PLAYER_RADIUS);
        assert_eq!(player.speed, PLAYER_SPEED);
        assert!(player.is_alive());
        assert_eq!(transform.translation.x, PLAYER_SPAWN_POS.x);
        assert_eq!(transform.translation.y, PLAYER_SPAWN_POS.y);
        assert_eq!(transform.translation.z, Z_PLAYER);
    }

    #[test]
    fn test_player_movement_arrow_keys() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .add_systems(Update, player_movement);

        // Spawn a player entity manually
        let start_pos = Vec2::new(0.0, 0.0);
        let player_entity = app
            .world_mut()
            .spawn((
                Player::default(),
                Transform::from_xyz(start_pos.x, start_pos.y, Z_PLAYER),
            ))
            .id();

        // 1. No input -> position unchanged
        app.update();
        let transform = app.world().get::<Transform>(player_entity).unwrap();
        assert_eq!(transform.translation.truncate(), start_pos);

        // 2. Press ArrowUp
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ArrowUp);

        // Advance time by 0.1s
        let dt = 0.1;
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs_f32(dt));

        app.update();

        let current_pos = {
            let transform = app.world().get::<Transform>(player_entity).unwrap();
            let expected_y = start_pos.y + PLAYER_SPEED * dt;
            assert!(
                (transform.translation.y - expected_y).abs() < 1e-4,
                "Player y should move up by speed * dt: expected {expected_y}, got {}",
                transform.translation.y
            );
            assert_eq!(transform.translation.x, start_pos.x);
            transform.translation.truncate()
        };

        // 3. Diagonal movement with ArrowRight and ArrowDown
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::ArrowRight);
        keys.press(KeyCode::ArrowDown);
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs_f32(dt));

        app.update();

        let transform = app.world().get::<Transform>(player_entity).unwrap();
        let expected_dir = Vec2::new(1.0, -1.0).normalize();
        let expected_delta = expected_dir * PLAYER_SPEED * dt;
        let actual_delta = transform.translation.truncate() - current_pos;

        assert!(
            (actual_delta.x - expected_delta.x).abs() < 1e-4,
            "Diagonal movement X mismatch"
        );
        assert!(
            (actual_delta.y - expected_delta.y).abs() < 1e-4,
            "Diagonal movement Y mismatch"
        );
    }

    #[test]
    fn test_player_movement_clamping() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .add_systems(Update, player_movement);

        // Start player at (380.0, 380.0), inside bounds (max is 386.0 with radius 14)
        let start_pos = Vec2::new(380.0, 380.0);
        let player_entity = app
            .world_mut()
            .spawn((
                Player::default(),
                Transform::from_xyz(start_pos.x, start_pos.y, Z_PLAYER),
            ))
            .id();

        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ArrowUp);
        keys.press(KeyCode::ArrowRight);

        // Advance 1.0 second: speed 180 would move to ~507, but clamped to 386
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs_f32(1.0));

        app.update();

        let transform = app.world().get::<Transform>(player_entity).unwrap();
        let max_x = WORLD_BOUNDS_MAX.x - PLAYER_RADIUS;
        let max_y = WORLD_BOUNDS_MAX.y - PLAYER_RADIUS;

        assert_eq!(transform.translation.x, max_x);
        assert_eq!(transform.translation.y, max_y);
    }

    #[test]
    fn test_dead_player_does_not_move() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .add_systems(Update, player_movement);

        let start_pos = Vec2::new(0.0, 0.0);
        let player_entity = app
            .world_mut()
            .spawn((
                Player {
                    radius: PLAYER_RADIUS,
                    speed: PLAYER_SPEED,
                    alive: false,
                },
                Transform::from_xyz(start_pos.x, start_pos.y, Z_PLAYER),
            ))
            .id();

        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ArrowUp);

        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs_f32(1.0));

        app.update();

        let transform = app.world().get::<Transform>(player_entity).unwrap();
        assert_eq!(transform.translation.truncate(), start_pos);
    }

    #[test]
    fn test_player_all_four_arrow_keys_responsiveness() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .add_systems(Update, player_movement);

        let start_pos = Vec2::new(0.0, 0.0);
        let player_entity = app
            .world_mut()
            .spawn((
                Player::default(),
                Transform::from_xyz(start_pos.x, start_pos.y, Z_PLAYER),
            ))
            .id();

        let dt = 0.1;
        let step = PLAYER_SPEED * dt;

        let directions = [
            (KeyCode::ArrowUp, Vec2::new(0.0, step)),
            (KeyCode::ArrowDown, Vec2::new(0.0, -step)),
            (KeyCode::ArrowLeft, Vec2::new(-step, 0.0)),
            (KeyCode::ArrowRight, Vec2::new(step, 0.0)),
        ];

        for (key, expected_delta) in directions {
            // Reset position to center
            let mut transform = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            transform.translation = Vec3::new(start_pos.x, start_pos.y, Z_PLAYER);

            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);

            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(dt));

            app.update();

            let transform = app.world().get::<Transform>(player_entity).unwrap();
            let actual_delta = transform.translation.truncate() - start_pos;
            assert!(
                (actual_delta.x - expected_delta.x).abs() < 1e-4,
                "Key {:?} X mismatch: expected {}, got {}",
                key, expected_delta.x, actual_delta.x
            );
            assert!(
                (actual_delta.y - expected_delta.y).abs() < 1e-4,
                "Key {:?} Y mismatch: expected {}, got {}",
                key, expected_delta.y, actual_delta.y
            );
        }
    }

    #[test]
    fn test_player_all_boundaries_clamping() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .add_systems(Update, player_movement);

        let player_entity = app
            .world_mut()
            .spawn((
                Player::default(),
                Transform::from_xyz(0.0, 0.0, Z_PLAYER),
            ))
            .id();

        let max_x = WORLD_BOUNDS_MAX.x - PLAYER_RADIUS;
        let min_x = WORLD_BOUNDS_MIN.x + PLAYER_RADIUS;
        let max_y = WORLD_BOUNDS_MAX.y - PLAYER_RADIUS;
        let min_y = WORLD_BOUNDS_MIN.y + PLAYER_RADIUS;

        let boundaries = [
            (KeyCode::ArrowLeft, min_x, true),
            (KeyCode::ArrowRight, max_x, true),
            (KeyCode::ArrowDown, min_y, false),
            (KeyCode::ArrowUp, max_y, false),
        ];

        for (key, expected_coord, is_x) in boundaries {
            // Place player close to the edge
            let mut transform = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
            transform.translation = Vec3::new(0.0, 0.0, Z_PLAYER);

            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);

            // Advance enough time to move way past boundary
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(5.0));

            app.update();

            let transform = app.world().get::<Transform>(player_entity).unwrap();
            if is_x {
                assert_eq!(transform.translation.x, expected_coord, "Boundary clamping X failed for {:?}", key);
            } else {
                assert_eq!(transform.translation.y, expected_coord, "Boundary clamping Y failed for {:?}", key);
            }
        }
    }
}
