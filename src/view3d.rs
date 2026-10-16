use crate::constants::*;
use bevy::prelude::*;
/// Converts 2D ground coordinates to 3D world coordinates.
///
/// Ground `(x, y)` maps to 3D `(x, h, -y)`:
/// +X stays +X (East), +Y becomes -Z (North), and height `h` is +Y (Up).
#[inline]
pub fn ground_to_3d(pos: Vec2, h: f32) -> Vec3 {
    Vec3::new(pos.x, h, -pos.y)
}

/// Converts a 2D heading angle (radians, CCW from +X) to a 3D rotation quaternion.
///
/// Heading `θ` maps to `Quat::from_rotation_y(θ)`:
/// A heading of 0 points along +X, and `π/2` points along -Z (North).
#[inline]
pub fn heading_to_3d(theta: f32) -> Quat {
    Quat::from_rotation_y(theta)
}

/// Marker component for the 3D perspective camera.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThreeDCamera;

/// Spawns the 3D perspective camera looking at the initial player ground position.
pub fn setup_camera_3d(mut commands: Commands) {
    let initial_x = PLAYER_SPAWN_POS.x.clamp(
        WORLD_BOUNDS_MIN.x + HALF_WINDOW_WIDTH,
        WORLD_BOUNDS_MAX.x - HALF_WINDOW_WIDTH,
    );
    let initial_y = PLAYER_SPAWN_POS.y.clamp(
        WORLD_BOUNDS_MIN.y + HALF_WINDOW_HEIGHT,
        WORLD_BOUNDS_MAX.y - HALF_WINDOW_HEIGHT,
    );
    let target = ground_to_3d(Vec2::new(initial_x, initial_y), 0.0);
    let cam_offset = Vec3::new(
        0.0,
        CAMERA3D_DISTANCE * CAMERA3D_PITCH.sin(),
        CAMERA3D_DISTANCE * CAMERA3D_PITCH.cos(),
    );
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: CAMERA3D_FOV,
            ..default()
        }),
        Transform::from_translation(target + cam_offset).looking_at(target, Vec3::Y),
        ThreeDCamera,
    ));
}

pub struct View3dPlugin;

impl Plugin for View3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera_3d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    #[test]
    fn test_ground_to_3d() {
        // +X stays +X
        let p_east = ground_to_3d(Vec2::new(100.0, 0.0), 10.0);
        assert_eq!(p_east, Vec3::new(100.0, 10.0, 0.0));

        // +Y becomes -Z
        let p_north = ground_to_3d(Vec2::new(0.0, 100.0), 10.0);
        assert_eq!(p_north, Vec3::new(0.0, 10.0, -100.0));

        // General point
        let p_general = ground_to_3d(Vec2::new(25.0, -75.0), 5.0);
        assert_eq!(p_general, Vec3::new(25.0, 5.0, 75.0));
    }

    #[test]
    fn test_heading_to_3d() {
        let car_long_axis = Vec3::X;

        // Heading 0: points along +X
        let dir_east = heading_to_3d(0.0) * car_long_axis;
        assert!(dir_east.distance(Vec3::X) < 1e-5);

        // Heading π/2: points a car's long axis toward -Z (North)
        let dir_north = heading_to_3d(FRAC_PI_2) * car_long_axis;
        assert!(
            dir_north.distance(-Vec3::Z) < 1e-5,
            "Heading π/2 must point long axis toward -Z (North), got {:?}",
            dir_north
        );

        // Heading π: points toward -X (West)
        let dir_west = heading_to_3d(PI) * car_long_axis;
        assert!(dir_west.distance(-Vec3::X) < 1e-5);

        // Heading 3π/2 or -π/2: points toward +Z (South)
        let dir_south = heading_to_3d(3.0 * FRAC_PI_2) * car_long_axis;
        assert!(dir_south.distance(Vec3::Z) < 1e-5);
    }

    #[test]
    fn test_setup_camera_3d_spawns_camera() {
        let mut app = App::new();
        app.add_systems(Startup, setup_camera_3d);
        app.update();

        let mut query = app
            .world_mut()
            .query::<(&Camera3d, &ThreeDCamera, &Projection, &Transform)>();
        let count = query.iter(app.world()).count();
        assert_eq!(count, 1, "Exactly one ThreeDCamera should be spawned");

        let (_, _, proj, transform) = query.iter(app.world()).next().unwrap();
        match proj {
            Projection::Perspective(p) => {
                assert_eq!(p.fov, CAMERA3D_FOV);
            }
            _ => panic!("Expected perspective projection"),
        }
        // Camera is elevated (+Y) and behind (+Z)
        assert!(transform.translation.y > 0.0);
    }
}
