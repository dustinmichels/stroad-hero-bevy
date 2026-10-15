use std::ops::{Deref, DerefMut};

use bevy::prelude::*;

use crate::constants::*;
use crate::road::RoadGeometry;
use crate::state::AppState;

/// Default vehicle colors for visual variety
pub const CAR_COLORS: [Color; 5] = [
    Color::srgb(0.85, 0.22, 0.22), // Crimson Red
    Color::srgb(0.22, 0.52, 0.88), // Royal Blue
    Color::srgb(0.95, 0.62, 0.12), // Amber Orange
    Color::srgb(0.92, 0.78, 0.16), // Taxi Yellow
    Color::srgb(0.58, 0.28, 0.78), // Purple
];

/// Enumeration of traffic lanes in the intersection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLane {
    Eastbound,
    Westbound,
    Southbound,
    Northbound,
}

impl TrafficLane {
    pub const ALL: [TrafficLane; 4] = [
        TrafficLane::Eastbound,
        TrafficLane::Westbound,
        TrafficLane::Southbound,
        TrafficLane::Northbound,
    ];

    /// Direction vector along the lane's travel line
    pub fn direction(&self) -> Vec2 {
        match self {
            TrafficLane::Eastbound => Vec2::new(1.0, 0.0),
            TrafficLane::Westbound => Vec2::new(-1.0, 0.0),
            TrafficLane::Southbound => Vec2::new(0.0, -1.0),
            TrafficLane::Northbound => Vec2::new(0.0, 1.0),
        }
    }

    /// Spawn origin located off-screen beyond the window margin
    pub fn spawn_position(&self, road_geom: &RoadGeometry) -> Vec3 {
        let spawn_x_dist = HALF_WINDOW_WIDTH + SPAWN_MARGIN;
        let spawn_y_dist = HALF_WINDOW_HEIGHT + SPAWN_MARGIN;

        match self {
            TrafficLane::Eastbound => Vec3::new(-spawn_x_dist, road_geom.east_lane_y(), Z_CAR),
            TrafficLane::Westbound => Vec3::new(spawn_x_dist, road_geom.west_lane_y(), Z_CAR),
            TrafficLane::Southbound => Vec3::new(road_geom.south_lane_x(), spawn_y_dist, Z_CAR),
            TrafficLane::Northbound => Vec3::new(road_geom.north_lane_x(), -spawn_y_dist, Z_CAR),
        }
    }

    /// Car bounding box dimensions (width along X, height along Y)
    pub fn car_dimensions(&self) -> Vec2 {
        match self {
            TrafficLane::Eastbound | TrafficLane::Westbound => Vec2::new(CAR_LENGTH, CAR_WIDTH),
            TrafficLane::Southbound | TrafficLane::Northbound => Vec2::new(CAR_WIDTH, CAR_LENGTH),
        }
    }
}

/// Component representing an active vehicle in the traffic system
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Car {
    /// Constant travel speed (units per second)
    pub speed: f32,
    /// Fixed unit direction vector (no steering or turning)
    pub direction: Vec2,
    /// Bounding box width along the X-axis
    pub width: f32,
    /// Bounding box height along the Y-axis
    pub height: f32,
}

impl Car {
    pub fn new(speed: f32, direction: Vec2, width: f32, height: f32) -> Self {
        Self {
            speed,
            direction: direction.normalize_or_zero(),
            width,
            height,
        }
    }

    pub fn dimensions(&self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    pub fn half_extents(&self) -> Vec2 {
        Vec2::new(self.width / 2.0, self.height / 2.0)
    }
}

/// Resource controlling periodic vehicle generation across intersection lanes
#[derive(Resource, Debug, Clone)]
pub struct SpawnTimer {
    pub timer: Timer,
    pub next_lane_index: usize,
}

impl Deref for SpawnTimer {
    type Target = Timer;

    fn deref(&self) -> &Self::Target {
        &self.timer
    }
}

impl DerefMut for SpawnTimer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.timer
    }
}

impl SpawnTimer {
    pub fn new(interval_secs: f32) -> Self {
        Self {
            timer: Timer::from_seconds(interval_secs, TimerMode::Repeating),
            next_lane_index: 0,
        }
    }

    /// Return the next lane and advance the internal lane index
    pub fn advance_lane(&mut self) -> TrafficLane {
        let lane = TrafficLane::ALL[self.next_lane_index % TrafficLane::ALL.len()];
        self.next_lane_index = (self.next_lane_index + 1) % TrafficLane::ALL.len();
        lane
    }
}

impl Default for SpawnTimer {
    fn default() -> Self {
        Self::new(CAR_SPAWN_INTERVAL_SECS)
    }
}

/// Helper function to check if a car's position has exited the opposite side of the screen
pub fn is_car_out_of_bounds(pos: Vec2, direction: Vec2) -> bool {
    let despawn_x = HALF_WINDOW_WIDTH + DESPAWN_MARGIN;
    let despawn_y = HALF_WINDOW_HEIGHT + DESPAWN_MARGIN;

    if direction.x > 0.0 && pos.x > despawn_x {
        return true;
    }
    if direction.x < 0.0 && pos.x < -despawn_x {
        return true;
    }
    if direction.y > 0.0 && pos.y > despawn_y {
        return true;
    }
    if direction.y < 0.0 && pos.y < -despawn_y {
        return true;
    }

    false
}

/// Spawns a single car in the specified lane
pub fn spawn_car_in_lane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    road_geom: &RoadGeometry,
    lane: TrafficLane,
    color_index: usize,
) -> Entity {
    let direction = lane.direction();
    let spawn_pos = lane.spawn_position(road_geom);
    let dims = lane.car_dimensions();

    let car = Car::new(CAR_SPEED, direction, dims.x, dims.y);
    let mesh = meshes.add(Rectangle::new(dims.x, dims.y));
    let color = CAR_COLORS[color_index % CAR_COLORS.len()];
    let material = materials.add(ColorMaterial::from_color(color));

    commands
        .spawn((
            Mesh2d(mesh),
            MeshMaterial2d(material),
            Transform::from_translation(spawn_pos),
            car,
        ))
        .id()
}

/// System to periodically generate cars across intersection lanes
pub fn spawn_cars(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<SpawnTimer>,
    road_geom: Res<RoadGeometry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if spawn_timer.timer.tick(time.delta()).just_finished() {
        let lane = spawn_timer.advance_lane();
        let color_idx = spawn_timer.next_lane_index;
        spawn_car_in_lane(
            &mut commands,
            &mut meshes,
            &mut materials,
            &road_geom,
            lane,
            color_idx,
        );
    }
}

/// System to advance each car's position along its fixed directional velocity
pub fn car_movement(time: Res<Time>, mut query: Query<(&Car, &mut Transform)>) {
    let dt = time.delta_secs();
    for (car, mut transform) in &mut query {
        let delta = car.direction * car.speed * dt;
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

/// System to despawn cars that have completely exited the opposite playfield boundary
pub fn car_cleanup(mut commands: Commands, query: Query<(Entity, &Car, &Transform)>) {
    for (entity, car, transform) in &query {
        let pos = transform.translation.truncate();
        if is_car_out_of_bounds(pos, car.direction) {
            commands.entity(entity).despawn();
        }
    }
}

/// Plugin registering traffic resources and systems
pub struct TrafficPlugin;

impl Plugin for TrafficPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnTimer>().add_systems(
            Update,
            (spawn_cars, car_movement, car_cleanup)
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[test]
    fn test_car_component_creation_and_bounds() {
        let car_h = Car::new(240.0, Vec2::new(1.0, 0.0), 48.0, 26.0);
        assert_eq!(car_h.speed, 240.0);
        assert_eq!(car_h.direction, Vec2::new(1.0, 0.0));
        assert_eq!(car_h.width, 48.0);
        assert_eq!(car_h.height, 26.0);
        assert_eq!(car_h.dimensions(), Vec2::new(48.0, 26.0));
        assert_eq!(car_h.half_extents(), Vec2::new(24.0, 13.0));

        let car_v = Car::new(240.0, Vec2::new(0.0, -1.0), 26.0, 48.0);
        assert_eq!(car_v.dimensions(), Vec2::new(26.0, 48.0));
        assert_eq!(car_v.half_extents(), Vec2::new(13.0, 24.0));
    }

    #[test]
    fn test_spawn_timer_advance_lane_cycles_all_lanes() {
        let mut timer = SpawnTimer::new(1.2);
        assert_eq!(timer.advance_lane(), TrafficLane::Eastbound);
        assert_eq!(timer.advance_lane(), TrafficLane::Westbound);
        assert_eq!(timer.advance_lane(), TrafficLane::Southbound);
        assert_eq!(timer.advance_lane(), TrafficLane::Northbound);
        assert_eq!(timer.advance_lane(), TrafficLane::Eastbound);
    }

    #[test]
    fn test_traffic_lane_directions_and_dimensions() {
        assert_eq!(TrafficLane::Eastbound.direction(), Vec2::new(1.0, 0.0));
        assert_eq!(TrafficLane::Westbound.direction(), Vec2::new(-1.0, 0.0));
        assert_eq!(TrafficLane::Southbound.direction(), Vec2::new(0.0, -1.0));
        assert_eq!(TrafficLane::Northbound.direction(), Vec2::new(0.0, 1.0));

        assert_eq!(
            TrafficLane::Eastbound.car_dimensions(),
            Vec2::new(CAR_LENGTH, CAR_WIDTH)
        );
        assert_eq!(
            TrafficLane::Westbound.car_dimensions(),
            Vec2::new(CAR_LENGTH, CAR_WIDTH)
        );
        assert_eq!(
            TrafficLane::Southbound.car_dimensions(),
            Vec2::new(CAR_WIDTH, CAR_LENGTH)
        );
        assert_eq!(
            TrafficLane::Northbound.car_dimensions(),
            Vec2::new(CAR_WIDTH, CAR_LENGTH)
        );
    }

    #[test]
    fn test_traffic_lane_spawn_positions() {
        let geom = RoadGeometry::default();
        let east_pos = TrafficLane::Eastbound.spawn_position(&geom);
        assert_eq!(east_pos.x, -HALF_WINDOW_WIDTH - SPAWN_MARGIN);
        assert_eq!(east_pos.y, geom.east_lane_y());
        assert_eq!(east_pos.z, Z_CAR);

        let west_pos = TrafficLane::Westbound.spawn_position(&geom);
        assert_eq!(west_pos.x, HALF_WINDOW_WIDTH + SPAWN_MARGIN);
        assert_eq!(west_pos.y, geom.west_lane_y());
        assert_eq!(west_pos.z, Z_CAR);

        let south_pos = TrafficLane::Southbound.spawn_position(&geom);
        assert_eq!(south_pos.x, geom.south_lane_x());
        assert_eq!(south_pos.y, HALF_WINDOW_HEIGHT + SPAWN_MARGIN);
        assert_eq!(south_pos.z, Z_CAR);

        let north_pos = TrafficLane::Northbound.spawn_position(&geom);
        assert_eq!(north_pos.x, geom.north_lane_x());
        assert_eq!(north_pos.y, -HALF_WINDOW_HEIGHT - SPAWN_MARGIN);
        assert_eq!(north_pos.z, Z_CAR);
    }

    #[test]
    fn test_is_car_out_of_bounds() {
        // Eastbound (heading +X)
        assert!(!is_car_out_of_bounds(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0)
        ));
        assert!(!is_car_out_of_bounds(
            Vec2::new(480.0, 0.0),
            Vec2::new(1.0, 0.0)
        ));
        assert!(is_car_out_of_bounds(
            Vec2::new(480.1, 0.0),
            Vec2::new(1.0, 0.0)
        ));

        // Westbound (heading -X)
        assert!(!is_car_out_of_bounds(
            Vec2::new(-480.0, 0.0),
            Vec2::new(-1.0, 0.0)
        ));
        assert!(is_car_out_of_bounds(
            Vec2::new(-480.1, 0.0),
            Vec2::new(-1.0, 0.0)
        ));

        // Northbound (heading +Y)
        assert!(!is_car_out_of_bounds(
            Vec2::new(0.0, 480.0),
            Vec2::new(0.0, 1.0)
        ));
        assert!(is_car_out_of_bounds(
            Vec2::new(0.0, 480.1),
            Vec2::new(0.0, 1.0)
        ));

        // Southbound (heading -Y)
        assert!(!is_car_out_of_bounds(
            Vec2::new(0.0, -480.0),
            Vec2::new(0.0, -1.0)
        ));
        assert!(is_car_out_of_bounds(
            Vec2::new(0.0, -480.1),
            Vec2::new(0.0, -1.0)
        ));
    }

    #[test]
    fn test_car_movement_system() {
        let mut app = App::new();
        app.init_resource::<Time>();
        // Spawn a horizontal and a vertical car
        let car_e = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::new(1.0, 0.0), 48.0, 26.0),
                Transform::from_xyz(0.0, -30.0, Z_CAR),
            ))
            .id();

        let car_s = app
            .world_mut()
            .spawn((
                Car::new(100.0, Vec2::new(0.0, -1.0), 26.0, 48.0),
                Transform::from_xyz(-30.0, 100.0, Z_CAR),
            ))
            .id();

        app.add_systems(Update, car_movement);

        // Advance 0.5s
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(500));

        app.update();

        let trans_e = app.world().get::<Transform>(car_e).unwrap();
        assert_eq!(trans_e.translation.x, 100.0); // 200 * 0.5
        assert_eq!(trans_e.translation.y, -30.0);

        let trans_s = app.world().get::<Transform>(car_s).unwrap();
        assert_eq!(trans_s.translation.x, -30.0);
        assert_eq!(trans_s.translation.y, 50.0); // 100 - 100 * 0.5
    }

    #[test]
    fn test_car_cleanup_system() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // In-bounds car
        let car_alive = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::new(1.0, 0.0), 48.0, 26.0),
                Transform::from_xyz(100.0, -30.0, Z_CAR),
            ))
            .id();

        // Exited car (+X eastbound past 480)
        let car_dead_e = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::new(1.0, 0.0), 48.0, 26.0),
                Transform::from_xyz(481.0, -30.0, Z_CAR),
            ))
            .id();

        // Exited car (-Y southbound past -480)
        let car_dead_s = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::new(0.0, -1.0), 26.0, 48.0),
                Transform::from_xyz(-30.0, -485.0, Z_CAR),
            ))
            .id();

        app.add_systems(Update, car_cleanup);
        app.update();

        assert!(app.world().get_entity(car_alive).is_ok());
        assert!(app.world().get_entity(car_dead_e).is_err());
        assert!(app.world().get_entity(car_dead_s).is_err());
    }

    #[test]
    fn test_car_spawning_adds_mesh_and_material() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<RoadGeometry>();

        let geom = RoadGeometry::default();

        type SpawnCarState<'w, 's> = bevy::ecs::system::SystemState<(
            Commands<'w, 's>,
            ResMut<'w, Assets<Mesh>>,
            ResMut<'w, Assets<ColorMaterial>>,
        )>;
        let car_id = {
            let mut system_state = SpawnCarState::new(app.world_mut());
            let (mut cmd, mut meshes, mut materials) =
                system_state.get_mut(app.world_mut()).unwrap();
            let id = spawn_car_in_lane(
                &mut cmd,
                &mut meshes,
                &mut materials,
                &geom,
                TrafficLane::Eastbound,
                0,
            );
            system_state.apply(app.world_mut());
            id
        };
        app.update();

        let car_ref = app
            .world()
            .get::<Car>(car_id)
            .expect("Car component should exist");
        assert_eq!(car_ref.direction, Vec2::new(1.0, 0.0));
        assert_eq!(car_ref.speed, CAR_SPEED);
        assert_eq!(car_ref.width, CAR_LENGTH);
        assert_eq!(car_ref.height, CAR_WIDTH);

        let transform = app
            .world()
            .get::<Transform>(car_id)
            .expect("Transform should exist");
        assert_eq!(transform.translation.z, Z_CAR);
        assert_eq!(transform.translation.y, geom.east_lane_y());
        assert_eq!(transform.translation.x, -HALF_WINDOW_WIDTH - SPAWN_MARGIN);

        assert!(app.world().get::<Mesh2d>(car_id).is_some());
        assert!(
            app.world()
                .get::<MeshMaterial2d<ColorMaterial>>(car_id)
                .is_some()
        );
    }

    #[test]
    fn test_traffic_plugin_periodic_spawn_in_app() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin)
            .init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<RoadGeometry>()
            .init_state::<AppState>()
            .add_plugins(TrafficPlugin);

        app.update();

        // Initially no cars spawned yet
        let initial_car_count = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(initial_car_count, 0);

        // Advance by spawn interval (1.2s)
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(1250));

        app.update();

        let spawned_cars = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(
            spawned_cars, 1,
            "Exactly one car should spawn after 1 interval"
        );
    }

    #[test]
    fn test_traffic_lifecycle_constant_velocity_and_no_leakage() {
        let dt = 0.05;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f32(dt),
            ))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<RoadGeometry>()
            .init_state::<AppState>()
            .add_plugins(TrafficPlugin);

        let duration_secs = 12.0;
        let total_frames = (duration_secs / dt) as usize;

        let mut observed_spawn_directions = std::collections::HashSet::new();
        let mut max_active_cars = 0;

        for _ in 0..total_frames {
            app.update();

            let mut car_query = app.world_mut().query::<(&Car, &Transform)>();
            let car_count = car_query.iter(app.world()).count();
            if car_count > max_active_cars {
                max_active_cars = car_count;
            }

            for (car, transform) in car_query.iter(app.world()) {
                assert!((car.direction.length() - 1.0).abs() < 1e-4);
                assert_eq!(car.speed, CAR_SPEED);
                observed_spawn_directions.insert((car.direction.x as i32, car.direction.y as i32));

                let pos = transform.translation.truncate();

                assert!(!is_car_out_of_bounds(pos, car.direction));
            }
        }

        assert_eq!(observed_spawn_directions.len(), 4, "All 4 directions should have spawned");
        assert!(max_active_cars >= 2, "Cars should have been active concurrently");
        assert!(max_active_cars <= 12, "Active cars should be bounded, got {}", max_active_cars);

        // Pause spawning and run until all existing cars despawn to prove zero entity leakage
        {
            let mut timer = app.world_mut().resource_mut::<SpawnTimer>();
            timer.set_duration(std::time::Duration::from_secs(1000));
            timer.reset();
        }

        for _ in 0..100 {

            app.update();
        }

        let remaining_cars = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(remaining_cars, 0, "All cars should have despawned cleanly without entity leakage");
    }
}
