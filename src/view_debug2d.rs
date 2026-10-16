use bevy::prelude::*;

use crate::collision;
use crate::constants::*;
use crate::player;
use crate::road;
use crate::sim;
use crate::state::GameSet;
use crate::traffic;

/// Marker component for the 2D top-down debug camera.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TopDownCamera;

pub struct DebugView2dPlugin;

impl Plugin for DebugView2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera).add_systems(
            Update,
            (
                camera_follow_player
                    .after(player::player_movement)
                    .in_set(GameSet::RenderSync),
                draw_debug_2d
                    .in_set(GameSet::DebugRender)
                    .run_if(in_state(crate::state::ViewMode::TopDown)),
            ),
        );
    }
}

/// Spawns the 2D orthographic camera for top-down debug viewing, tagged with `TopDownCamera`.
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
        Camera {
            is_active: false,
            ..default()
        },
        Transform::from_xyz(initial_cam_x, initial_cam_y, 0.0),
        TopDownCamera,
    ));
}

/// Smoothly scrolls the top-down camera to follow the player while clamping within world bounds.
pub fn camera_follow_player(
    player_query: Query<&sim::GroundPos, (With<player::Player>, Without<TopDownCamera>)>,
    mut camera_query: Query<&mut Transform, (With<TopDownCamera>, With<Camera2d>)>,
) {
    let Ok(ground_pos) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let px = ground_pos.x;
    let py = ground_pos.y;

    let min_cam_x = WORLD_BOUNDS_MIN.x + HALF_WINDOW_WIDTH;
    let max_cam_x = WORLD_BOUNDS_MAX.x - HALF_WINDOW_WIDTH;
    let min_cam_y = WORLD_BOUNDS_MIN.y + HALF_WINDOW_HEIGHT;
    let max_cam_y = WORLD_BOUNDS_MAX.y - HALF_WINDOW_HEIGHT;

    let target_x = px.clamp(min_cam_x, max_cam_x);
    let target_y = py.clamp(min_cam_y, max_cam_y);

    if camera_transform.translation.x != target_x || camera_transform.translation.y != target_y {
        camera_transform.translation.x = target_x;
        camera_transform.translation.y = target_y;
    }
}

/// Top-down 2D debug drawing system using Gizmos.
///
/// Draws:
/// - Road outlines from `road_pieces` (asphalt, borders, centerlines, stop lines)
/// - Compact signal-state markers at each approach curb
/// - Player circle colored green when alive, red when dead
/// - Car boxes as rotated rectangles from `GroundPos`, `Heading`, and footprint,
///   with braking indicators and crashed crosses
/// - X marker for each blood splat
#[allow(clippy::type_complexity)]
pub fn draw_debug_2d(
    mut gizmos: Gizmos,
    cam_query: Query<&Camera, With<TopDownCamera>>,
    signal: Option<Res<traffic::TrafficSignal>>,
    road_geom: Option<Res<road::RoadGeometry>>,
    players: Query<(&sim::GroundPos, &player::Player)>,
    cars: Query<(
        &traffic::Car,
        &sim::GroundPos,
        &sim::Heading,
        Option<&traffic::CarMotion>,
        Has<traffic::Crashed>,
    )>,
    splats: Query<&sim::GroundPos, With<collision::BloodSplat>>,
) {
    if let Ok(cam) = cam_query.single()
        && !cam.is_active
    {
        return;
    }
    let geom = road_geom.as_deref().cloned().unwrap_or_default();

    // 1. Road outlines from road_pieces
    for piece in road::road_pieces(&geom) {
        let color = match piece.kind {
            road::RoadPieceKind::Asphalt => road::COLOR_ROAD_ASPHALT,
            road::RoadPieceKind::Border => road::COLOR_ROAD_BORDER,
            road::RoadPieceKind::Centerline => road::COLOR_ROAD_CENTERLINE,
            road::RoadPieceKind::StopLine => road::COLOR_ROAD_STOP_LINE,
        };
        gizmos.rect_2d(
            Isometry2d::new(piece.center, Rot2::radians(piece.angle)),
            piece.size,
            color,
        );
    }

    // 2. Compact signal-state markers at each approach
    if let Some(signal) = signal {
        let aspect_color = match signal.state {
            traffic::TrafficLightState::Red => COLOR_SIGNAL_RED,
            traffic::TrafficLightState::Yellow => COLOR_SIGNAL_YELLOW,
            traffic::TrafficLightState::Green => COLOR_SIGNAL_GREEN,
        };
        for lane in traffic::TrafficLane::ALL {
            let pos = geom.signal_position(lane);
            gizmos.circle_2d(pos, 7.5, COLOR_SIGNAL_HOUSING);
            gizmos.circle_2d(pos, 6.0, aspect_color);
        }
    }

    // 3. Blood splat X markers
    for ground_pos in &splats {
        let half_s = 8.0;
        let pos = ground_pos.0;
        gizmos.line_2d(
            pos + Vec2::new(-half_s, -half_s),
            pos + Vec2::new(half_s, half_s),
            COLOR_BLOOD_SPLAT,
        );
        gizmos.line_2d(
            pos + Vec2::new(-half_s, half_s),
            pos + Vec2::new(half_s, -half_s),
            COLOR_BLOOD_SPLAT,
        );
    }

    // 4. Car boxes as rotated rectangles from GroundPos/Heading/footprint
    for (car, ground_pos, heading, maybe_motion, is_crashed) in &cars {
        let footprint = Vec2::new(car.length, car.width);
        let iso = Isometry2d::new(ground_pos.0, Rot2::radians(heading.0));
        let car_color = if is_crashed {
            traffic::COLOR_CAR_CRASHED
        } else {
            traffic::CAR_COLORS[car.color_index % traffic::CAR_COLORS.len()]
        };
        gizmos.rect_2d(iso, footprint, car_color);

        let visual_state =
            maybe_motion
                .map(|m| m.visual_state(is_crashed))
                .unwrap_or(if is_crashed {
                    traffic::CarVisualState::Crashed
                } else {
                    traffic::CarVisualState::Normal
                });

        match visual_state {
            traffic::CarVisualState::Crashed => {
                let outline_size = Vec2::new(car.length + 4.0, car.width + 4.0);
                gizmos.rect_2d(iso, outline_size, traffic::COLOR_CAR_CRASHED);

                let half_l = car.length * 0.4;
                let half_w = car.width * 0.4;
                let p1 = ground_pos.0 + iso.rotation * Vec2::new(-half_l, -half_w);
                let p2 = ground_pos.0 + iso.rotation * Vec2::new(half_l, half_w);
                let p3 = ground_pos.0 + iso.rotation * Vec2::new(-half_l, half_w);
                let p4 = ground_pos.0 + iso.rotation * Vec2::new(half_l, -half_w);
                gizmos.line_2d(p1, p2, traffic::COLOR_CAR_CRASHED);
                gizmos.line_2d(p3, p4, traffic::COLOR_CAR_CRASHED);
            }
            traffic::CarVisualState::Braking => {
                let rear_x = -car.length / 2.0;
                let half_w = car.width / 2.0;
                let left_light = ground_pos.0 + iso.rotation * Vec2::new(rear_x, -half_w * 0.7);
                let right_light = ground_pos.0 + iso.rotation * Vec2::new(rear_x, half_w * 0.7);
                gizmos.circle_2d(left_light, 3.0, traffic::COLOR_CAR_BRAKING);
                gizmos.circle_2d(right_light, 3.0, traffic::COLOR_CAR_BRAKING);
            }
            traffic::CarVisualState::Normal => {}
        }
    }

    // 5. Player circle colored by alive state
    for (ground_pos, player) in &players {
        let color = if player.alive {
            player::COLOR_PLAYER
        } else {
            player::COLOR_PLAYER_DEAD
        };
        gizmos.circle_2d(ground_pos.0, PLAYER_RADIUS, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_spawn_has_top_down_camera() {
        let mut app = App::new();
        app.add_systems(Startup, setup_camera);
        app.update();

        let mut camera_query = app
            .world_mut()
            .query::<(&Camera2d, &TopDownCamera, &Camera, &Transform)>();
        let mut count = 0;
        for (_, _, camera, transform) in camera_query.iter(app.world()) {
            count += 1;
            assert!(!camera.is_active, "Camera2d should start inactive");
            assert_eq!(transform.translation.x, -160.0);
            assert_eq!(transform.translation.y, -100.0);
        }
        assert_eq!(count, 1, "Expected exactly 1 TopDownCamera");
    }

    #[test]
    fn test_camera_follow_player_tracks_and_clamps() {
        let mut app = App::new();
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, camera_follow_player);

        let player_entity = app
            .world_mut()
            .spawn((
                player::Player::default(),
                sim::GroundPos(Vec2::new(-160.0, -160.0)),
            ))
            .id();

        app.update();

        let mut cam_query = app.world_mut().query::<(&TopDownCamera, &Transform)>();
        let (_, cam_transform) = cam_query.iter(app.world()).next().unwrap();
        assert_eq!(cam_transform.translation.x, -160.0);
        assert_eq!(cam_transform.translation.y, -100.0);

        // Move player to (300.0, 50.0)
        let mut gp = app
            .world_mut()
            .get_mut::<sim::GroundPos>(player_entity)
            .unwrap();
        gp.0 = Vec2::new(300.0, 50.0);
        app.update();

        let mut cam_query = app.world_mut().query::<(&TopDownCamera, &Transform)>();
        let (_, cam_transform) = cam_query.iter(app.world()).next().unwrap();
        assert_eq!(cam_transform.translation.x, 300.0);
        assert_eq!(cam_transform.translation.y, 50.0);

        // Move player beyond camera max: (750.0, 50.0)
        let mut gp = app
            .world_mut()
            .get_mut::<sim::GroundPos>(player_entity)
            .unwrap();
        gp.0 = Vec2::new(750.0, 50.0);
        app.update();

        let mut cam_query = app.world_mut().query::<(&TopDownCamera, &Transform)>();
        let (_, cam_transform) = cam_query.iter(app.world()).next().unwrap();
        let max_cam_x = WORLD_BOUNDS_MAX.x - HALF_WINDOW_WIDTH;
        assert_eq!(cam_transform.translation.x, max_cam_x);
    }

    #[test]
    fn test_draw_debug_2d_executes_without_panic() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
        .init_resource::<road::RoadGeometry>()
        .insert_resource(traffic::TrafficSignal::default())
        .add_systems(Update, draw_debug_2d);

        // Spawn TopDownCamera as active
        app.world_mut().spawn((
            Camera2d,
            Camera {
                is_active: true,
                ..default()
            },
            TopDownCamera,
        ));

        // Spawn player (alive)
        app.world_mut().spawn((
            player::Player {
                alive: true,
                ..default()
            },
            sim::GroundPos(Vec2::new(10.0, 20.0)),
        ));

        // Spawn player (dead)
        app.world_mut().spawn((
            player::Player {
                alive: false,
                ..default()
            },
            sim::GroundPos(Vec2::new(-50.0, -50.0)),
        ));

        // Spawn normal car
        app.world_mut().spawn((
            traffic::Car {
                speed: 100.0,
                direction: Vec2::X,
                length: 40.0,
                width: 20.0,
                lane: Some(traffic::TrafficLane::Eastbound),
                color_index: 0,
                committed_to_cross: false,
            },
            sim::GroundPos(Vec2::new(100.0, 0.0)),
            sim::Heading(0.0),
        ));

        // Spawn braking car
        let mut motion = traffic::CarMotion::new(100.0);
        motion.current_speed = 50.0;
        motion.desired_speed = 0.0;
        app.world_mut().spawn((
            traffic::Car {
                speed: 100.0,
                direction: Vec2::X,
                length: 40.0,
                width: 20.0,
                lane: Some(traffic::TrafficLane::Eastbound),
                color_index: 1,
                committed_to_cross: false,
            },
            motion,
            sim::GroundPos(Vec2::new(200.0, 0.0)),
            sim::Heading(0.0),
        ));

        // Spawn crashed car
        app.world_mut().spawn((
            traffic::Car {
                speed: 100.0,
                direction: Vec2::X,
                length: 40.0,
                width: 20.0,
                lane: Some(traffic::TrafficLane::Eastbound),
                color_index: 2,
                committed_to_cross: false,
            },
            sim::GroundPos(Vec2::new(300.0, 0.0)),
            sim::Heading(0.0),
            traffic::Crashed {
                timer: Timer::from_seconds(1.0, TimerMode::Once),
            },
        ));

        // Spawn blood splat
        app.world_mut().spawn((
            collision::BloodSplat { seed: 12345 },
            sim::GroundPos(Vec2::new(15.0, 25.0)),
        ));

        // Update should execute draw_debug_2d successfully
        app.update();
    }

    #[test]
    fn test_draw_debug_2d_skips_when_camera_inactive() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
        .add_systems(Update, draw_debug_2d);
        // Spawn inactive TopDownCamera
        app.world_mut().spawn((
            Camera2d,
            Camera {
                is_active: false,
                ..default()
            },
            TopDownCamera,
        ));

        // Update should safely return early
        app.update();
    }

    #[test]
    fn test_draw_debug_2d_gated_by_view_mode_top_down() {
        use crate::state::ViewMode;
        use bevy::state::app::StatesPlugin;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
        .init_state::<ViewMode>()
        .add_plugins(DebugView2dPlugin);

        // In default ViewMode::ThreeD, plugin is built and draw_debug_2d is gated by TopDown
        app.update();

        // Switch to TopDown
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::TopDown);
        app.update();
        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::TopDown
        );
    }
}
