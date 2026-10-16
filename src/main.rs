use bevy::{prelude::*, window::WindowResolution};

pub mod collision;
pub mod constants;
pub mod player;
pub mod road;
pub mod spd;
pub mod state;
pub mod traffic;
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
    .add_systems(Startup, setup_camera);
    app
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
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
        assert_eq!(road_count, 2);
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
            .add_systems(Startup, setup_camera);
        app.update();

        assert!(app.world().contains_resource::<spd::Spd>());
        assert!(app.world().contains_resource::<road::RoadGeometry>());
        assert!(app.world().contains_resource::<traffic::SpawnTimer>());
        assert!(app.world().contains_resource::<ButtonInput<KeyCode>>());
    }
}
