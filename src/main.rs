use bevy::{prelude::*, window::WindowResolution};

pub mod collision;
pub mod constants;
pub mod player;
pub mod road;
pub mod sim;
pub mod spd;
pub mod state;
pub mod traffic;
pub mod view3d;
use constants::*;
use state::AppState;

fn main() {
    build_app().run();
}

pub fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Stroad Hero".into(),
            resolution: WindowResolution::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32),
            resizable: false,
            canvas: Some("#game".into()),
            ..default()
        }),
        ..default()
    }))
    .init_state::<AppState>()
    .add_plugins(road::RoadPlugin)
    .add_plugins(player::PlayerPlugin)
    .add_plugins(traffic::TrafficPlugin)
    .add_plugins(collision::CollisionPlugin)
    .add_plugins(spd::SpdPlugin)
    .add_plugins(view3d::View3dPlugin)
    .add_systems(Startup, setup_camera)
    .add_systems(
        Update,
        (
            sync_2d
                .after(player::player_movement)
                .after(traffic::car_movement)
                .after(collision::collision_detection)
                .after(collision::restart_game),
            camera_follow_player.after(sync_2d),
        ),
    );
    app
}

pub fn setup_camera(mut commands: Commands) {
    let initial_cam_x = PLAYER_SPAWN_POS.x.clamp(
        WORLD_BOUNDS_MIN.x + HALF_WINDOW_WIDTH,
        WORLD_BOUNDS_MAX.x - HALF_WINDOW_WIDTH,
    );
    let initial_cam_y = PLAYER_SPAWN_POS.y.clamp(
        WORLD_BOUNDS_MIN.y + HALF_WINDOW_HEIGHT,
        WORLD_BOUNDS_MAX.y - HALF_WINDOW_HEIGHT,
    );
    commands.spawn((
        Camera2d,
        Transform::from_xyz(initial_cam_x, initial_cam_y, 0.0),
    ));
}

/// System to smoothly scroll the camera to follow the player while clamping within world bounds
pub fn camera_follow_player(
    player_query: Query<&Transform, (With<player::Player>, Without<Camera2d>)>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let min_cam_x = WORLD_BOUNDS_MIN.x + HALF_WINDOW_WIDTH;
    let max_cam_x = WORLD_BOUNDS_MAX.x - HALF_WINDOW_WIDTH;
    let min_cam_y = WORLD_BOUNDS_MIN.y + HALF_WINDOW_HEIGHT;
    let max_cam_y = WORLD_BOUNDS_MAX.y - HALF_WINDOW_HEIGHT;

    let target_x = player_transform.translation.x.clamp(min_cam_x, max_cam_x);
    let target_y = player_transform.translation.y.clamp(min_cam_y, max_cam_y);

    camera_transform.translation.x = target_x;
    camera_transform.translation.y = target_y;
}

/// Temporary bridge system that syncs 2D Transform from GroundPos and Heading for existing meshes.
/// Section 7 of the 3D plan deletes this once 3D rendering owns transforms.
pub fn sync_2d(mut query: Query<(&sim::GroundPos, Option<&sim::Heading>, &mut Transform)>) {
    for (ground_pos, heading, mut transform) in &mut query {
        transform.translation.x = ground_pos.x;
        transform.translation.y = ground_pos.y;
        if let Some(heading) = heading {
            transform.rotation = Quat::from_rotation_z(heading.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[test]
    fn test_camera_spawn() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .add_systems(Startup, setup_camera);
        app.update();

        let mut camera_query = app.world_mut().query::<&Camera2d>();
        let camera_count = camera_query.iter(app.world()).count();
        assert_eq!(camera_count, 1, "Camera2d should be spawned on startup");
    }

    #[test]
    fn test_camera_follow_player_tracks_and_clamps() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup_camera)
            .add_systems(Update, camera_follow_player);

        let player_entity = app
            .world_mut()
            .spawn((
                player::Player::default(),
                Transform::from_xyz(-160.0, -160.0, Z_PLAYER),
            ))
            .id();

        app.update();

        // Initial camera position matches spawn clamped
        let mut cam_query = app.world_mut().query::<(&Camera2d, &Transform)>();
        let (_, cam_transform) = cam_query.iter(app.world()).next().unwrap();
        assert_eq!(cam_transform.translation.x, -160.0);
        assert_eq!(cam_transform.translation.y, -100.0); // clamped to min_cam_y = -500 + 400 = -100

        // Move player into previously off-screen area: x = 300.0, y = 50.0
        let mut p_transform = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_transform.translation = Vec3::new(300.0, 50.0, Z_PLAYER);

        app.update();

        let mut cam_query = app.world_mut().query::<(&Camera2d, &Transform)>();
        let (_, cam_transform) = cam_query.iter(app.world()).next().unwrap();
        assert_eq!(cam_transform.translation.x, 300.0);
        assert_eq!(cam_transform.translation.y, 50.0);

        // Move player to extreme right beyond camera max: x = 750.0
        let mut p_transform = app.world_mut().get_mut::<Transform>(player_entity).unwrap();
        p_transform.translation = Vec3::new(750.0, 50.0, Z_PLAYER);

        app.update();

        let mut cam_query = app.world_mut().query::<(&Camera2d, &Transform)>();
        let (_, cam_transform) = cam_query.iter(app.world()).next().unwrap();
        let max_cam_x = WORLD_BOUNDS_MAX.x - HALF_WINDOW_WIDTH; // 800 - 400 = 400
        assert_eq!(cam_transform.translation.x, max_cam_x);
    }

    #[test]
    fn test_initial_state_in_app() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>();
        app.update();

        let state = app.world().resource::<State<AppState>>();
        assert_eq!(*state.get(), AppState::InGame);
    }

    #[test]
    fn test_road_setup_in_app() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .add_plugins(road::RoadPlugin);
        app.update();

        assert!(app.world().contains_resource::<road::RoadGeometry>());
        let road_count = app
            .world_mut()
            .query::<&road::Road>()
            .iter(app.world())
            .count();
        assert_eq!(road_count, 3);
    }

    #[test]
    fn test_player_setup_in_app() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_state::<AppState>()
            .add_plugins(player::PlayerPlugin);
        app.update();

        let player_count = app
            .world_mut()
            .query::<&player::Player>()
            .iter(app.world())
            .count();
        assert_eq!(player_count, 1);
    }

    #[test]
    fn test_traffic_setup_in_app() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<road::RoadGeometry>()
            .init_state::<AppState>()
            .add_plugins(traffic::TrafficPlugin);
        app.update();

        assert!(app.world().contains_resource::<traffic::SpawnTimer>());
    }

    #[test]
    fn test_collision_setup_in_app() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_state::<AppState>()
            .add_plugins(collision::CollisionPlugin);
        app.update();

        assert!(app.world().contains_resource::<ButtonInput<KeyCode>>());
    }

    #[test]
    fn test_spd_setup_in_app() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .add_plugins(spd::SpdPlugin);
        app.update();

        assert!(app.world().contains_resource::<spd::Spd>());
    }

    #[test]
    fn test_full_integrated_app_smoke() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_state::<AppState>()
            .add_plugins(road::RoadPlugin)
            .add_plugins(player::PlayerPlugin)
            .add_plugins(traffic::TrafficPlugin)
            .add_plugins(collision::CollisionPlugin)
            .add_plugins(spd::SpdPlugin)
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (
                    sync_2d
                        .after(player::player_movement)
                        .after(traffic::car_movement)
                        .after(collision::collision_detection)
                        .after(collision::restart_game),
                    camera_follow_player.after(sync_2d),
                ),
            );
        app.update();

        assert!(app.world().contains_resource::<spd::Spd>());
        assert!(app.world().contains_resource::<road::RoadGeometry>());
        assert!(app.world().contains_resource::<traffic::SpawnTimer>());
        assert!(app.world().contains_resource::<ButtonInput<KeyCode>>());

        // Player starts at spawn
        let mut p_query = app
            .world_mut()
            .query::<(&player::Player, &sim::GroundPos)>();
        let (_, p_pos) = p_query.iter(app.world()).next().unwrap();
        assert_eq!(p_pos.0, PLAYER_SPAWN_POS);

        // Move player towards off-screen curved road section (e.g. x = 400.0, y = 200.0)
        let player_entity = app
            .world_mut()
            .query_filtered::<Entity, With<player::Player>>()
            .iter(app.world())
            .next()
            .unwrap();
        let mut p_pos = app
            .world_mut()
            .get_mut::<sim::GroundPos>(player_entity)
            .unwrap();
        p_pos.0 = Vec2::new(400.0, 200.0);

        app.update();

        // Camera tracks the player into previously off-screen territory
        let mut cam_query = app.world_mut().query::<(&Camera2d, &Transform)>();
        let (_, cam_trans) = cam_query.iter(app.world()).next().unwrap();
        assert_eq!(cam_trans.translation.x, 400.0);
        assert_eq!(cam_trans.translation.y, 100.0); // clamped to max_cam_y = 100.0
    }

    #[test]
    fn test_sync_2d_updates_transform() {
        let mut app = App::new();
        app.add_systems(Update, sync_2d);

        let entity_with_heading = app
            .world_mut()
            .spawn((
                sim::GroundPos::new(120.0, -45.0),
                sim::Heading::new(std::f32::consts::FRAC_PI_2),
                Transform::from_xyz(0.0, 0.0, Z_CAR),
            ))
            .id();

        let entity_without_heading = app
            .world_mut()
            .spawn((
                sim::GroundPos::new(30.0, 60.0),
                Transform::from_xyz(0.0, 0.0, Z_BLOOD_SPLAT),
            ))
            .id();

        app.update();

        let t1 = app.world().get::<Transform>(entity_with_heading).unwrap();
        assert_eq!(t1.translation, Vec3::new(120.0, -45.0, Z_CAR));
        assert_eq!(
            t1.rotation,
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
        );

        let t2 = app
            .world()
            .get::<Transform>(entity_without_heading)
            .unwrap();
        assert_eq!(t2.translation, Vec3::new(30.0, 60.0, Z_BLOOD_SPLAT));
        assert_eq!(t2.rotation, Quat::IDENTITY);
    }
}
