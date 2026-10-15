use bevy::prelude::*;

// Window and Virtual World Dimensions
pub const WINDOW_WIDTH: f32 = 800.0;
pub const WINDOW_HEIGHT: f32 = 800.0;

pub const HALF_WINDOW_WIDTH: f32 = WINDOW_WIDTH / 2.0;
pub const HALF_WINDOW_HEIGHT: f32 = WINDOW_HEIGHT / 2.0;

pub const WORLD_BOUNDS_MIN: Vec2 = Vec2::new(-HALF_WINDOW_WIDTH, -HALF_WINDOW_HEIGHT);
pub const WORLD_BOUNDS_MAX: Vec2 = Vec2::new(HALF_WINDOW_WIDTH, HALF_WINDOW_HEIGHT);

// Grid and Road Dimensions
pub const GRID_STEP_SIZE: f32 = 40.0;
pub const ROAD_WIDTH: f32 = 120.0;
pub const LANE_WIDTH: f32 = ROAD_WIDTH / 2.0; // 60.0

// Z-Ordering Layers
pub const Z_ROAD: f32 = 0.0;
pub const Z_CAR: f32 = 1.0;
pub const Z_PLAYER: f32 = 2.0;

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
        assert_eq!(WORLD_BOUNDS_MIN.x, -400.0);
        assert_eq!(WORLD_BOUNDS_MIN.y, -400.0);
        assert_eq!(WORLD_BOUNDS_MAX.x, 400.0);
        assert_eq!(WORLD_BOUNDS_MAX.y, 400.0);
    }

    #[test]
    fn test_lane_and_road_geometry() {
        assert_eq!(ROAD_WIDTH, 120.0);
        assert_eq!(LANE_WIDTH, 60.0);
        assert!(GRID_STEP_SIZE > 0.0);
    }

    #[test]
    fn test_z_ordering() {
        assert!(Z_ROAD < Z_CAR);
        assert!(Z_CAR < Z_PLAYER);
    }

    #[test]
    fn test_speeds_and_intervals() {
        assert!(PLAYER_SPEED > 0.0);
        assert!(CAR_SPEED > 0.0);
        assert!(CAR_SPAWN_INTERVAL_SECS > 0.0);
    }
}
