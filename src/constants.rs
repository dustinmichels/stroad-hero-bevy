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

// Z-Ordering Layers
pub const Z_ROAD: f32 = 0.0;
pub const Z_BLOOD_SPLAT: f32 = 0.05;
pub const Z_CAR: f32 = 1.0;
pub const Z_PLAYER: f32 = 2.0;

// Blood Splat Appearance
pub const COLOR_BLOOD_SPLAT: Color = Color::srgba(0.80, 0.38, 0.40, 0.85);
// Movement Speeds
pub const PLAYER_SPEED: f32 = 180.0;
pub const CAR_SPEED: f32 = 240.0;

// Player Dimensions
pub const PLAYER_RADIUS: f32 = 14.0;
pub const PLAYER_SPAWN_POS: Vec2 = Vec2::new(-160.0, -160.0);

// Vehicle Dimensions and Spawning
pub const CAR_LENGTH: f32 = 48.0;
pub const CAR_WIDTH: f32 = 26.0;
pub const CAR_SPAWN_INTERVAL_SECS: f32 = 1.2;
pub const SPAWN_MARGIN: f32 = 60.0;
pub const DESPAWN_MARGIN: f32 = 80.0;

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
        assert!(GRID_STEP_SIZE > 0.0);
    }

    #[test]
    fn test_z_ordering() {
        assert!(Z_ROAD < Z_BLOOD_SPLAT);
        assert!(Z_BLOOD_SPLAT < Z_CAR);
        assert!(Z_CAR < Z_PLAYER);
    }

    #[test]
    fn test_speeds_and_intervals() {
        assert!(PLAYER_SPEED > 0.0);
        assert!(CAR_SPEED > 0.0);
        assert!(CAR_SPAWN_INTERVAL_SECS > 0.0);
    }
}
