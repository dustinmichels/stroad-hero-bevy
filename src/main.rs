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
pub mod view_debug2d;
pub mod view_toggle;
use constants::*;
use state::{AppState, GameSet, ViewMode};

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
    .init_state::<ViewMode>()
    .configure_sets(
        Update,
        (
            GameSet::Input,
            GameSet::Sim,
            GameSet::Collision,
            GameSet::Cleanup,
            GameSet::RenderSync,
            GameSet::DebugRender,
        )
            .chain(),
    )
    .add_plugins(road::RoadPlugin)
    .add_plugins(player::PlayerPlugin)
    .add_plugins(traffic::TrafficPlugin)
    .add_plugins(collision::CollisionPlugin)
    .add_plugins(spd::SpdPlugin)
    .add_plugins(view3d::View3dPlugin)
    .add_plugins(view_debug2d::DebugView2dPlugin)
    .add_plugins(view_toggle::ViewTogglePlugin);
    app
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
            .add_systems(Startup, view_debug2d::setup_camera);
        app.update();

        let mut camera_query = app
            .world_mut()
            .query::<(&Camera2d, &view_debug2d::TopDownCamera)>();
        let camera_count = camera_query.iter(app.world()).count();
        assert_eq!(
            camera_count, 1,
            "TopDownCamera Camera2d should be spawned by setup_camera"
        );
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
    fn test_initial_view_mode_in_app() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<ViewMode>();
        app.update();

        let state = app.world().resource::<State<ViewMode>>();
        assert_eq!(*state.get(), ViewMode::ThreeD);
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
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            AssetPlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
        .init_state::<AppState>()
        .init_state::<ViewMode>()
        .add_plugins(road::RoadPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(traffic::TrafficPlugin)
        .add_plugins(collision::CollisionPlugin)
        .add_plugins(spd::SpdPlugin)
        .add_plugins(view3d::View3dPlugin)
        .add_plugins(view_debug2d::DebugView2dPlugin)
        .add_plugins(view_toggle::ViewTogglePlugin);
        app.update();

        assert!(app.world().contains_resource::<spd::Spd>());
        assert!(app.world().contains_resource::<road::RoadGeometry>());
        assert!(app.world().contains_resource::<traffic::SpawnTimer>());
        assert!(app.world().contains_resource::<ButtonInput<KeyCode>>());
        assert!(app.world().contains_resource::<view3d::CarAssets3d>());
        assert!(app.world().contains_resource::<view3d::SignalAssets3d>());

        // Both cameras exist, and initially ThreeDCamera is active while TopDownCamera is inactive
        let mut cam3d_query = app
            .world_mut()
            .query::<(&view3d::ThreeDCamera, &Camera, &Transform)>();
        let (_, cam3d_cam, _) = cam3d_query.iter(app.world()).next().unwrap();
        assert!(cam3d_cam.is_active, "3D camera should be active initially");

        let mut cam2d_query = app
            .world_mut()
            .query::<(&view_debug2d::TopDownCamera, &Camera, &Transform)>();
        let (_, cam2d_cam, _) = cam2d_query.iter(app.world()).next().unwrap();
        assert!(
            !cam2d_cam.is_active,
            "2D camera should be inactive initially"
        );

        // Player starts at spawn in simulation, and 3D player proxy is spawned
        let mut p_query = app
            .world_mut()
            .query::<(&player::Player, &sim::GroundPos)>();
        let (_, p_pos) = p_query.iter(app.world()).next().unwrap();
        assert_eq!(p_pos.0, PLAYER_SPAWN_POS);

        let mut p3d_query = app.world_mut().query::<(&view3d::Player3d, &Transform)>();
        let (_, p3d_trans) = p3d_query.iter(app.world()).next().unwrap();
        let half_height = PLAYER_HEIGHT_3D / 2.0 + PLAYER_RADIUS;
        assert_eq!(
            p3d_trans.translation,
            view3d::ground_to_3d(PLAYER_SPAWN_POS, half_height)
        );

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

        // 3D player proxy updates translation
        let mut p3d_query = app.world_mut().query::<(&view3d::Player3d, &Transform)>();
        let (_, p3d_trans) = p3d_query.iter(app.world()).next().unwrap();
        assert_eq!(
            p3d_trans.translation,
            view3d::ground_to_3d(Vec2::new(400.0, 200.0), half_height)
        );

        // 2D Camera tracks the player into previously off-screen territory and clamps
        let mut cam_query = app
            .world_mut()
            .query::<(&view_debug2d::TopDownCamera, &Transform)>();
        let (_, cam_trans) = cam_query.iter(app.world()).next().unwrap();
        assert_eq!(cam_trans.translation.x, 400.0);
        assert_eq!(cam_trans.translation.y, 100.0); // clamped to max_cam_y = 100.0

        // 3D Camera tracks the player clamped target
        let mut cam3d_query = app
            .world_mut()
            .query::<(&view3d::ThreeDCamera, &Transform)>();
        let (_, cam3d_trans) = cam3d_query.iter(app.world()).next().unwrap();
        let expected_target = view3d::ground_to_3d(Vec2::new(400.0, 100.0), 0.0);
        let expected_offset = Vec3::new(
            0.0,
            CAMERA3D_DISTANCE * CAMERA3D_PITCH.sin(),
            CAMERA3D_DISTANCE * CAMERA3D_PITCH.cos(),
        );
        assert!(
            cam3d_trans
                .translation
                .distance(expected_target + expected_offset)
                < 1e-4,
            "3D camera must track clamped player position"
        );

        // Switch view to TopDown
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::TopDown);
        app.update();

        let mut cam3d_query = app.world_mut().query::<(&view3d::ThreeDCamera, &Camera)>();
        let (_, cam3d_cam) = cam3d_query.iter(app.world()).next().unwrap();
        assert!(
            !cam3d_cam.is_active,
            "3D camera should be inactive after switch to TopDown"
        );

        let mut cam2d_query = app
            .world_mut()
            .query::<(&view_debug2d::TopDownCamera, &Camera)>();
        let (_, cam2d_cam) = cam2d_query.iter(app.world()).next().unwrap();
        assert!(
            cam2d_cam.is_active,
            "2D camera should be active after switch to TopDown"
        );
    }

    #[test]
    fn test_simulation_independence_collisions_and_spd_identical_across_view_modes() {
        fn build_sim_test_app(mode: ViewMode) -> App {
            let mut app = App::new();
            app.add_plugins((
                MinimalPlugins,
                StatesPlugin,
                AssetPlugin::default(),
                bevy::gizmos::GizmoPlugin,
            ))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
            .init_state::<AppState>()
            .init_state::<ViewMode>()
            .add_plugins(road::RoadPlugin)
            .add_plugins(player::PlayerPlugin)
            .add_plugins(traffic::TrafficPlugin)
            .add_plugins(collision::CollisionPlugin)
            .add_plugins(spd::SpdPlugin)
            .add_plugins(view3d::View3dPlugin)
            .add_plugins(view_debug2d::DebugView2dPlugin)
            .add_plugins(view_toggle::ViewTogglePlugin);

            if mode != ViewMode::ThreeD {
                app.world_mut()
                    .resource_mut::<NextState<ViewMode>>()
                    .set(mode);
            }
            app.update();
            assert_eq!(*app.world().resource::<State<ViewMode>>().get(), mode);
            app
        }

        let mut app_3d = build_sim_test_app(ViewMode::ThreeD);
        let mut app_2d = build_sim_test_app(ViewMode::TopDown);

        // Verify initial SPD is 0 in both apps
        assert_eq!(app_3d.world().resource::<spd::Spd>().0, 0);
        assert_eq!(app_2d.world().resource::<spd::Spd>().0, 0);

        // Move player to (0.0, 50.0) in both apps
        let player_hit_pos = Vec2::new(0.0, 50.0);
        for app in [&mut app_3d, &mut app_2d] {
            let p_entity = app
                .world_mut()
                .query_filtered::<Entity, With<player::Player>>()
                .iter(app.world())
                .next()
                .unwrap();
            app.world_mut()
                .get_mut::<sim::GroundPos>(p_entity)
                .unwrap()
                .0 = player_hit_pos;
        }

        // Spawn a car directly overlapping player in both apps
        for app in [&mut app_3d, &mut app_2d] {
            app.world_mut().spawn((
                sim::GroundPos(player_hit_pos),
                sim::Heading::new(0.0),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ));
        }

        // Run simulation step
        app_3d.update();
        app_2d.update();

        // 1. Both apps must detect collision, increment SPD identically (0 -> 1),
        // reset player to PLAYER_SPAWN_POS identically, and spawn blood splat identically.
        let spd_3d = app_3d.world().resource::<spd::Spd>().0;
        let spd_2d = app_2d.world().resource::<spd::Spd>().0;
        assert_eq!(spd_3d, 1);
        assert_eq!(spd_2d, 1);
        assert_eq!(
            spd_3d, spd_2d,
            "SPD must increment identically in both ViewModes"
        );

        let p_pos_3d = app_3d
            .world_mut()
            .query_filtered::<&sim::GroundPos, With<player::Player>>()
            .iter(app_3d.world())
            .next()
            .unwrap()
            .0;
        let p_pos_2d = app_2d
            .world_mut()
            .query_filtered::<&sim::GroundPos, With<player::Player>>()
            .iter(app_2d.world())
            .next()
            .unwrap()
            .0;
        assert_eq!(p_pos_3d, PLAYER_SPAWN_POS);
        assert_eq!(p_pos_2d, PLAYER_SPAWN_POS);

        let splats_3d: Vec<_> = app_3d
            .world_mut()
            .query::<(&collision::BloodSplat, &sim::GroundPos)>()
            .iter(app_3d.world())
            .collect();
        let splats_2d: Vec<_> = app_2d
            .world_mut()
            .query::<(&collision::BloodSplat, &sim::GroundPos)>()
            .iter(app_2d.world())
            .collect();
        assert_eq!(splats_3d.len(), 1);
        assert_eq!(splats_2d.len(), 1);
        assert_eq!(splats_3d[0].0.seed, splats_2d[0].0.seed);
        assert_eq!(splats_3d[0].1.0, splats_2d[0].1.0);

        // 2. Second collision test: move player into collision again to verify second increment
        let player_hit_pos_2 = Vec2::new(100.0, 50.0);
        for app in [&mut app_3d, &mut app_2d] {
            let p_entity = app
                .world_mut()
                .query_filtered::<Entity, With<player::Player>>()
                .iter(app.world())
                .next()
                .unwrap();
            app.world_mut()
                .get_mut::<sim::GroundPos>(p_entity)
                .unwrap()
                .0 = player_hit_pos_2;

            app.world_mut().spawn((
                sim::GroundPos(player_hit_pos_2),
                sim::Heading::new(0.0),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ));
        }

        app_3d.update();
        app_2d.update();

        assert_eq!(app_3d.world().resource::<spd::Spd>().0, 2);
        assert_eq!(app_2d.world().resource::<spd::Spd>().0, 2);

        // 3. Vehicle rear-end collision test:
        // Spawn a stopped lead car and a follower overlapping in the Eastbound lane
        let east_y = app_3d
            .world()
            .resource::<road::RoadGeometry>()
            .east_lane_y();
        let lead_pos = Vec2::new(100.0, east_y);
        let follower_pos = Vec2::new(55.0, east_y); // Leader rear = 76.0, Follower front = 79.0 -> gap <= 0

        for app in [&mut app_3d, &mut app_2d] {
            // Lead car stopped
            app.world_mut().spawn((
                sim::GroundPos(lead_pos),
                sim::Heading::ZERO,
                traffic::Car::new(0.0, Vec2::X, CAR_LENGTH, CAR_WIDTH)
                    .with_lane(traffic::TrafficLane::Eastbound),
                traffic::CarMotion::new(0.0),
            ));
            // Follower car
            app.world_mut().spawn((
                sim::GroundPos(follower_pos),
                sim::Heading::ZERO,
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH)
                    .with_lane(traffic::TrafficLane::Eastbound),
                traffic::CarMotion::new(CAR_SPEED),
            ));
        }

        app_3d.update();
        app_2d.update();

        let crashes_3d = app_3d
            .world_mut()
            .query_filtered::<Entity, With<traffic::Crashed>>()
            .iter(app_3d.world())
            .count();
        let crashes_2d = app_2d
            .world_mut()
            .query_filtered::<Entity, With<traffic::Crashed>>()
            .iter(app_2d.world())
            .count();
        assert!(
            crashes_3d > 0,
            "Rear-end collision must trigger crash in 3D"
        );
        assert_eq!(
            crashes_3d, crashes_2d,
            "Vehicle crash detections must match between ViewMode::ThreeD and ViewMode::TopDown"
        );

        // 4. Toggle view modes dynamically: app_3d -> TopDown, app_2d -> ThreeD
        app_3d
            .world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::TopDown);
        app_2d
            .world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::ThreeD);
        app_3d.update();
        app_2d.update();

        assert_eq!(
            *app_3d.world().resource::<State<ViewMode>>().get(),
            ViewMode::TopDown
        );
        assert_eq!(
            *app_2d.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );

        // Move player into a third collision after dynamic mode toggle
        let player_hit_pos_3 = Vec2::new(150.0, 50.0);
        for app in [&mut app_3d, &mut app_2d] {
            let p_entity = app
                .world_mut()
                .query_filtered::<Entity, With<player::Player>>()
                .iter(app.world())
                .next()
                .unwrap();
            app.world_mut()
                .get_mut::<sim::GroundPos>(p_entity)
                .unwrap()
                .0 = player_hit_pos_3;

            app.world_mut().spawn((
                sim::GroundPos(player_hit_pos_3),
                sim::Heading::new(0.0),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ));
        }

        app_3d.update();
        app_2d.update();

        assert_eq!(app_3d.world().resource::<spd::Spd>().0, 3);
        assert_eq!(app_2d.world().resource::<spd::Spd>().0, 3);
    }

    #[test]
    fn test_native_smoke_play_toggle_crash_restart() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            AssetPlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
        .init_state::<AppState>()
        .init_state::<ViewMode>()
        .add_plugins(road::RoadPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(traffic::TrafficPlugin)
        .add_plugins(collision::CollisionPlugin)
        .add_plugins(spd::SpdPlugin)
        .add_plugins(view3d::View3dPlugin)
        .add_plugins(view_debug2d::DebugView2dPlugin)
        .add_plugins(view_toggle::ViewTogglePlugin);

        app.update();

        // 1. Initial State: Play in 3D
        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );
        let mut cam3d_query = app.world_mut().query::<(&view3d::ThreeDCamera, &Camera)>();
        let (_, cam3d_cam) = cam3d_query.iter(app.world()).next().unwrap();
        assert!(cam3d_cam.is_active, "3D camera should be active initially");

        let mut cam2d_query = app
            .world_mut()
            .query::<(&view_debug2d::TopDownCamera, &Camera)>();
        let (_, cam2d_cam) = cam2d_query.iter(app.world()).next().unwrap();
        assert!(
            !cam2d_cam.is_active,
            "2D camera should be inactive initially"
        );

        // Player starts at spawn pos
        let p_entity = app
            .world_mut()
            .query_filtered::<Entity, With<player::Player>>()
            .iter(app.world())
            .next()
            .unwrap();
        let p_pos = app.world().get::<sim::GroundPos>(p_entity).unwrap().0;
        assert_eq!(p_pos, PLAYER_SPAWN_POS);

        // Move player forward (WASD/arrows: ArrowUp / KeyW)
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyW);
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.1));
        }
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyW);
        }

        let p_pos_after = app.world().get::<sim::GroundPos>(p_entity).unwrap().0;
        assert!(
            p_pos_after.y > p_pos.y,
            "Player should move forward with W key"
        );

        // Spawn traffic car
        let car_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(-200.0, 50.0)),
                sim::Heading::new(0.0),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH)
                    .with_lane(traffic::TrafficLane::Eastbound),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        app.update();

        // Verify 3D proxies match GroundPos
        let p3d_trans = *app
            .world_mut()
            .query_filtered::<&Transform, With<view3d::Player3d>>()
            .iter(app.world())
            .next()
            .unwrap();
        let half_height = PLAYER_HEIGHT_3D / 2.0 + PLAYER_RADIUS;
        assert_eq!(
            p3d_trans.translation,
            view3d::ground_to_3d(p_pos_after, half_height)
        );

        let car3d_trans = *app
            .world_mut()
            .query_filtered::<&Transform, With<view3d::Car3d>>()
            .iter(app.world())
            .next()
            .unwrap();
        let car_pos = app.world().get::<sim::GroundPos>(car_entity).unwrap().0;
        assert_eq!(
            car3d_trans.translation,
            view3d::ground_to_3d(car_pos, CAR_HEIGHT_3D / 2.0)
        );

        // 2. Switch to debug 2D mid-traffic via 'V'
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyV);
        }
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear_just_pressed(KeyCode::KeyV);
            keys.release(KeyCode::KeyV);
        }
        app.update();

        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::TopDown
        );
        let cam3d_active = {
            let mut query = app.world_mut().query::<(&view3d::ThreeDCamera, &Camera)>();
            query.iter(app.world()).next().unwrap().1.is_active
        };
        let cam2d_active = {
            let mut query = app
                .world_mut()
                .query::<(&view_debug2d::TopDownCamera, &Camera)>();
            query.iter(app.world()).next().unwrap().1.is_active
        };
        assert!(!cam3d_active, "3D camera should be inactive in TopDown");
        assert!(cam2d_active, "2D camera should be active in TopDown");

        // 3. Confirm boxes/circles match positions
        let current_p_pos = app.world().get::<sim::GroundPos>(p_entity).unwrap().0;
        let current_car_pos = app.world().get::<sim::GroundPos>(car_entity).unwrap().0;
        let cur_p3d_trans = *app
            .world_mut()
            .query_filtered::<&Transform, With<view3d::Player3d>>()
            .iter(app.world())
            .next()
            .unwrap();
        let cur_car3d_trans = *app
            .world_mut()
            .query_filtered::<&Transform, With<view3d::Car3d>>()
            .iter(app.world())
            .next()
            .unwrap();
        assert_eq!(current_p_pos.x, cur_p3d_trans.translation.x);
        assert_eq!(current_p_pos.y, -cur_p3d_trans.translation.z);
        assert_eq!(current_car_pos.x, cur_car3d_trans.translation.x);
        assert_eq!(current_car_pos.y, -cur_car3d_trans.translation.z);

        // 4. Toggle back to 3D via 'V'
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyV);
        }
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear_just_pressed(KeyCode::KeyV);
            keys.release(KeyCode::KeyV);
        }
        app.update();

        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );
        let cam3d_active = {
            let mut query = app.world_mut().query::<(&view3d::ThreeDCamera, &Camera)>();
            query.iter(app.world()).next().unwrap().1.is_active
        };
        let cam2d_active = {
            let mut query = app
                .world_mut()
                .query::<(&view_debug2d::TopDownCamera, &Camera)>();
            query.iter(app.world()).next().unwrap().1.is_active
        };
        assert!(cam3d_active, "3D camera should be active in ThreeD");
        assert!(!cam2d_active, "2D camera should be inactive in ThreeD");

        // 5. Crash: Move player directly into traffic car
        app.world_mut()
            .get_mut::<sim::GroundPos>(p_entity)
            .unwrap()
            .0 = current_car_pos;
        app.update();

        // Verify crash results: SPD incremented, blood splat spawned, player reset to spawn
        assert_eq!(
            app.world().resource::<spd::Spd>().0,
            1,
            "SPD should increment on collision"
        );
        let splat_count = app
            .world_mut()
            .query_filtered::<Entity, With<collision::BloodSplat>>()
            .iter(app.world())
            .count();
        assert_eq!(splat_count, 1, "Blood splat must spawn on crash");
        let reset_p_pos = app.world().get::<sim::GroundPos>(p_entity).unwrap().0;
        assert_eq!(
            reset_p_pos, PLAYER_SPAWN_POS,
            "Player resets to spawn on collision"
        );

        // 6. Restart: Press 'R'
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyR);
        }
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyR);
        }

        // Verify restart state: SPD reset to 0, blood splats despawned, cars despawned
        assert_eq!(
            app.world().resource::<spd::Spd>().0,
            0,
            "SPD must reset to 0 on restart"
        );
        let splat_count_after = app
            .world_mut()
            .query_filtered::<Entity, With<collision::BloodSplat>>()
            .iter(app.world())
            .count();
        assert_eq!(
            splat_count_after, 0,
            "Blood splats should be cleared on restart"
        );
        let car_count_after = app
            .world_mut()
            .query_filtered::<Entity, With<traffic::Car>>()
            .iter(app.world())
            .count();
        assert_eq!(car_count_after, 0, "Cars should be cleared on restart");
        let final_p_pos = app.world().get::<sim::GroundPos>(p_entity).unwrap().0;
        assert_eq!(final_p_pos, PLAYER_SPAWN_POS, "Player must remain at spawn");
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );
    }

    #[test]
    fn test_game_set_schedule_ordering() {
        #[derive(Resource, Default)]
        struct ExecutionLog(Vec<&'static str>);

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .init_state::<ViewMode>()
            .configure_sets(
                Update,
                (
                    GameSet::Input,
                    GameSet::Sim,
                    GameSet::Collision,
                    GameSet::Cleanup,
                    GameSet::RenderSync,
                    GameSet::DebugRender,
                )
                    .chain(),
            )
            .init_resource::<ExecutionLog>()
            .add_systems(
                Update,
                (
                    (|mut log: ResMut<ExecutionLog>| log.0.push("Input")).in_set(GameSet::Input),
                    (|mut log: ResMut<ExecutionLog>| log.0.push("Sim")).in_set(GameSet::Sim),
                    (|mut log: ResMut<ExecutionLog>| log.0.push("Collision"))
                        .in_set(GameSet::Collision),
                    (|mut log: ResMut<ExecutionLog>| log.0.push("Cleanup"))
                        .in_set(GameSet::Cleanup),
                    (|mut log: ResMut<ExecutionLog>| log.0.push("RenderSync"))
                        .in_set(GameSet::RenderSync),
                    (|mut log: ResMut<ExecutionLog>| log.0.push("DebugRender"))
                        .in_set(GameSet::DebugRender),
                ),
            );

        app.update();

        let log = app.world().resource::<ExecutionLog>();
        assert_eq!(
            log.0,
            vec![
                "Input",
                "Sim",
                "Collision",
                "Cleanup",
                "RenderSync",
                "DebugRender"
            ],
            "GameSets must execute in strictly ordered pipeline"
        );
    }
}
