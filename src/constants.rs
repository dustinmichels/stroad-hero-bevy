use bevy::prelude::*;

// Window Dimensions (Visible Screen Viewport)
pub const WINDOW_WIDTH: f32 = 800.0;
pub const WINDOW_HEIGHT: f32 = 800.0;

pub const HALF_WINDOW_WIDTH: f32 = WINDOW_WIDTH / 2.0;
pub const HALF_WINDOW_HEIGHT: f32 = WINDOW_HEIGHT / 2.0;

// Virtual World / Map Dimensions (Larger than window viewport, partially off-screen)
pub const WORLD_WIDTH: f32 = 1600.0;
pub const WORLD_HEIGHT: f32 = 1000.0;

pub const HALF_WORLD_WIDTH: f32 = WORLD_WIDTH / 2.0;
pub const HALF_WORLD_HEIGHT: f32 = WORLD_HEIGHT / 2.0;

pub const WORLD_BOUNDS_MIN: Vec2 = Vec2::new(-HALF_WORLD_WIDTH, -HALF_WORLD_HEIGHT);
pub const WORLD_BOUNDS_MAX: Vec2 = Vec2::new(HALF_WORLD_WIDTH, HALF_WORLD_HEIGHT);

// Road Curve Constants (Eastbound/Westbound road bends North into the off-screen area)
pub const CURVE_CENTER: Vec2 = Vec2::new(160.0, 500.0);
pub const CURVE_START_X: f32 = CURVE_CENTER.x;
pub const CURVE_RADIUS: f32 = 500.0;
// Grid and Road Dimensions
pub const GRID_STEP_SIZE: f32 = 40.0;
pub const ROAD_WIDTH: f32 = 120.0;
pub const LANE_WIDTH: f32 = ROAD_WIDTH / 2.0; // 60.0

// Blood Splat Appearance
pub const COLOR_BLOOD_SPLAT: Color = Color::srgba(0.80, 0.38, 0.40, 0.85);
// Movement Speeds
pub const PLAYER_SPEED: f32 = 180.0;
pub const CAR_SPEED: f32 = 240.0;
pub const CAR_ACCELERATION: f32 = 160.0;
pub const CAR_COMFORTABLE_DECELERATION: f32 = 300.0;
pub const CAR_EMERGENCY_DECELERATION: f32 = 520.0;
pub const DRIVER_REACTION_SECS: f32 = 0.35;
pub const CAR_MIN_FOLLOW_GAP: f32 = 14.0;
// Player Dimensions
pub const PLAYER_RADIUS: f32 = 14.0;
pub const PLAYER_SPAWN_POS: Vec2 = Vec2::new(-160.0, -160.0);

// Vehicle Dimensions and Spawning
pub const CAR_LENGTH: f32 = 48.0;
pub const CAR_WIDTH: f32 = 26.0;
pub const CAR_SPAWN_INTERVAL_SECS: f32 = 1.2;
pub const SPAWN_MARGIN: f32 = 60.0;
pub const DESPAWN_MARGIN: f32 = 80.0;
pub const STOP_LINE_BUFFER: f32 = 4.0;
// Crash Lifecycle Tuning Constants
pub const CRASH_CLEANUP_SECS: f32 = 3.0;
// Traffic Signal Phase Tuning Constants
pub const SIGNAL_GREEN_SECS: f32 = 6.0;
pub const SIGNAL_YELLOW_SECS: f32 = 1.5;
pub const SIGNAL_RED_SECS: f32 = 5.0;
// 3D View Constants
pub const PLAYER_HEIGHT_3D: f32 = 30.0;
pub const CAR_HEIGHT_3D: f32 = 20.0;
pub const CAMERA3D_PITCH: f32 = 50.0 * std::f32::consts::PI / 180.0;
pub const CAMERA3D_DISTANCE: f32 = 850.0;
pub const CAMERA3D_FOV: f32 = 45.0 * std::f32::consts::PI / 180.0;
pub const COLOR_GROUND_GRASS: Color = Color::srgb(0.22, 0.45, 0.22);
pub const GROUND_PLANE_WIDTH: f32 = WORLD_WIDTH + 2400.0;
pub const GROUND_PLANE_HEIGHT: f32 = WORLD_HEIGHT + 2400.0;
// 3D Signal Head Constants
pub const SIGNAL_POLE_HEIGHT: f32 = 45.0;
pub const SIGNAL_POLE_RADIUS: f32 = 1.5;
pub const SIGNAL_HOUSING_HEIGHT: f32 = 18.0;
pub const SIGNAL_HOUSING_WIDTH: f32 = 6.0;
pub const SIGNAL_HOUSING_DEPTH: f32 = 4.0;
pub const SIGNAL_LENS_RADIUS: f32 = 1.8;
pub const COLOR_SIGNAL_HOUSING: Color = Color::srgb(0.18, 0.18, 0.20);
pub const COLOR_SIGNAL_POLE: Color = Color::srgb(0.35, 0.35, 0.38);
pub const COLOR_SIGNAL_RED: Color = Color::srgb(0.95, 0.15, 0.15);
pub const COLOR_SIGNAL_YELLOW: Color = Color::srgb(0.95, 0.80, 0.10);
pub const COLOR_SIGNAL_GREEN: Color = Color::srgb(0.15, 0.90, 0.25);
pub const COLOR_SIGNAL_OFF: Color = Color::srgb(0.12, 0.12, 0.14);
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_bounds() {
        assert_eq!(WORLD_WIDTH, 1600.0);
        assert_eq!(WORLD_HEIGHT, 1000.0);
        assert_eq!(WORLD_BOUNDS_MIN.x, -800.0);
        assert_eq!(WORLD_BOUNDS_MIN.y, -500.0);
        assert_eq!(WORLD_BOUNDS_MAX.x, 800.0);
        assert_eq!(WORLD_BOUNDS_MAX.y, 500.0);
    }

    #[test]
    fn test_curve_constants() {
        assert_eq!(CURVE_START_X, 160.0);
        assert_eq!(CURVE_CENTER, Vec2::new(160.0, 500.0));
        assert_eq!(CURVE_RADIUS, 500.0);
    }

    #[test]
    fn test_lane_and_road_geometry() {
        assert_eq!(ROAD_WIDTH, 120.0);
        assert_eq!(LANE_WIDTH, 60.0);
        const {
            assert!(GRID_STEP_SIZE > 0.0);
        }
    }

    #[test]
    fn test_speeds_and_intervals() {
        const {
            assert!(PLAYER_SPEED > 0.0);
            assert!(CAR_SPEED > 0.0);
            assert!(CAR_SPAWN_INTERVAL_SECS > 0.0);
        }
    }

    #[test]
    fn test_car_motion_tuning_constants() {
        assert_eq!(CAR_ACCELERATION, 160.0);
        assert_eq!(CAR_COMFORTABLE_DECELERATION, 300.0);
        assert_eq!(CAR_EMERGENCY_DECELERATION, 520.0);
        assert_eq!(DRIVER_REACTION_SECS, 0.35);
        assert_eq!(CAR_MIN_FOLLOW_GAP, 14.0);
        const {
            assert!(CAR_COMFORTABLE_DECELERATION > CAR_ACCELERATION);
            assert!(CAR_EMERGENCY_DECELERATION > CAR_COMFORTABLE_DECELERATION);
            assert!(DRIVER_REACTION_SECS > 0.0);
            assert!(CAR_MIN_FOLLOW_GAP > 0.0);
        }
    }

    #[test]
    fn test_signal_constants() {
        assert_eq!(SIGNAL_GREEN_SECS, 6.0);
        assert_eq!(SIGNAL_YELLOW_SECS, 1.5);
        assert_eq!(SIGNAL_RED_SECS, 5.0);
    }

    #[test]
    fn test_crash_cleanup_constant() {
        assert_eq!(CRASH_CLEANUP_SECS, 3.0);
        const {
            assert!(CRASH_CLEANUP_SECS > 0.0);
        }
    }

    #[test]
    fn test_3d_constants() {
        const {
            assert!(PLAYER_HEIGHT_3D > 0.0);
            assert!(CAR_HEIGHT_3D > 0.0);
            assert!(CAMERA3D_PITCH > 0.0 && CAMERA3D_PITCH < std::f32::consts::FRAC_PI_2);
            assert!(CAMERA3D_DISTANCE > 0.0);
            assert!(CAMERA3D_FOV > 0.0 && CAMERA3D_FOV < std::f32::consts::PI);
            assert!(GROUND_PLANE_WIDTH >= WORLD_WIDTH);
            assert!(GROUND_PLANE_HEIGHT >= WORLD_HEIGHT);
        }
    }
}
