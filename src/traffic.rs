use std::ops::{Deref, DerefMut};

use bevy::prelude::*;

use crate::constants::*;
use crate::road::RoadGeometry;
use crate::sim::{GroundPos, Heading};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        let spawn_x_dist = HALF_WORLD_WIDTH + SPAWN_MARGIN;
        let spawn_y_dist = HALF_WORLD_HEIGHT + SPAWN_MARGIN;

        match self {
            TrafficLane::Eastbound => Vec3::new(-spawn_x_dist, road_geom.east_lane_y(), Z_CAR),
            TrafficLane::Westbound => Vec3::new(road_geom.curve_west_lane_x(), spawn_y_dist, Z_CAR),
            TrafficLane::Southbound => Vec3::new(road_geom.south_lane_x(), spawn_y_dist, Z_CAR),
            TrafficLane::Northbound => Vec3::new(road_geom.north_lane_x(), -spawn_y_dist, Z_CAR),
        }
    }

    /// Car bounding box dimensions (length along heading, width perpendicular to heading)
    pub fn car_dimensions(&self) -> Vec2 {
        Vec2::new(CAR_LENGTH, CAR_WIDTH)
    }

    /// Initial spawn heading for a vehicle entering this lane
    pub fn spawn_heading(&self) -> Heading {
        match self {
            TrafficLane::Eastbound => Heading::ZERO,
            TrafficLane::Westbound => Heading::new(-std::f32::consts::FRAC_PI_2),
            TrafficLane::Southbound => Heading::new(-std::f32::consts::FRAC_PI_2),
            TrafficLane::Northbound => Heading::new(std::f32::consts::FRAC_PI_2),
        }
    }
}

/// Component representing an active vehicle in the traffic system
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Car {
    /// Constant travel speed (units per second)
    pub speed: f32,
    /// Fixed or current unit direction vector
    pub direction: Vec2,
    /// Bounding box length along heading
    pub length: f32,
    /// Bounding box width perpendicular to heading
    pub width: f32,
    /// Assigned traffic lane, if spawned via traffic system
    pub lane: Option<TrafficLane>,
}

impl Car {
    pub fn new(speed: f32, direction: Vec2, length: f32, width: f32) -> Self {
        Self {
            speed,
            direction: direction.normalize_or_zero(),
            length,
            width,
            lane: None,
        }
    }

    pub fn with_lane(mut self, lane: TrafficLane) -> Self {
        self.lane = Some(lane);
        self
    }

    pub fn dimensions(&self) -> Vec2 {
        Vec2::new(self.length, self.width)
    }

    pub fn half_extents(&self) -> Vec2 {
        Vec2::new(self.length / 2.0, self.width / 2.0)
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
    let despawn_x = HALF_WORLD_WIDTH + DESPAWN_MARGIN;
    let despawn_y = HALF_WORLD_HEIGHT + DESPAWN_MARGIN;

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
    let initial_dir = match lane {
        TrafficLane::Westbound => Vec2::new(0.0, -1.0),
        _ => lane.direction(),
    };
    let spawn_pos = lane.spawn_position(road_geom);
    let dims = lane.car_dimensions();

    let car = Car::new(CAR_SPEED, initial_dir, dims.x, dims.y).with_lane(lane);
    let mesh = meshes.add(Rectangle::new(dims.x, dims.y));
    let color = CAR_COLORS[color_index % CAR_COLORS.len()];
    let material = materials.add(ColorMaterial::from_color(color));

    let heading = lane.spawn_heading();
    let rotation = Quat::from_rotation_z(heading.0);

    commands
        .spawn((
            Mesh2d(mesh),
            MeshMaterial2d(material),
            Transform::from_translation(spawn_pos).with_rotation(rotation),
            GroundPos(spawn_pos.truncate()),
            heading,
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

pub fn car_movement(
    time: Res<Time>,
    road_geom: Option<Res<RoadGeometry>>,
    mut query: Query<(&mut Car, &mut GroundPos, &mut Heading)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    let default_geom = RoadGeometry::default();
    let geom = road_geom.as_deref().unwrap_or(&default_geom);
    let curve_start_x = geom.curve_start_x;
    let curve_center = geom.curve_center;
    let r_east = geom.curve_east_lane_radius();
    let r_west = geom.curve_west_lane_radius();
    let east_lane_y = geom.east_lane_y();
    let west_lane_y = geom.west_lane_y();
    let south_lane_x = geom.south_lane_x();
    let north_lane_x = geom.north_lane_x();

    for (mut car, mut ground_pos, mut heading) in &mut query {
        match car.lane {
            Some(TrafficLane::Eastbound) => {
                let pos = ground_pos.0;
                if pos.x < curve_start_x {
                    // Straight horizontal segment heading East
                    let next_x = pos.x + car.speed * dt;
                    if next_x < curve_start_x {
                        ground_pos.x = next_x;
                        ground_pos.y = east_lane_y;
                        car.direction = Vec2::X;
                        heading.0 = 0.0;
                    } else {
                        // Transitions into curve
                        let excess_s = next_x - curve_start_x;
                        let theta = excess_s / r_east;
                        ground_pos.x = curve_start_x + r_east * theta.sin();
                        ground_pos.y = curve_center.y - r_east * theta.cos();
                        car.direction = Vec2::new(theta.cos(), theta.sin());
                        heading.0 = theta;
                    }
                } else if pos.y < curve_center.y {
                    // On the curve heading towards the top exit
                    let current_theta = (pos.x - curve_start_x).atan2(curve_center.y - pos.y);
                    let d_theta = (car.speed / r_east) * dt;
                    let next_theta = current_theta + d_theta;

                    if next_theta < std::f32::consts::FRAC_PI_2 {
                        ground_pos.x = curve_start_x + r_east * next_theta.sin();
                        ground_pos.y = curve_center.y - r_east * next_theta.cos();
                        car.direction = Vec2::new(next_theta.cos(), next_theta.sin());
                        heading.0 = next_theta;
                    } else {
                        // Exits curve to the North
                        let excess_s = (next_theta - std::f32::consts::FRAC_PI_2) * r_east;
                        ground_pos.x = curve_start_x + r_east;
                        ground_pos.y = curve_center.y + excess_s;
                        car.direction = Vec2::Y;
                        heading.0 = std::f32::consts::FRAC_PI_2;
                    }
                } else {
                    // Past the curve heading straight North
                    ground_pos.y += car.speed * dt;
                    ground_pos.x = curve_start_x + r_east;
                    car.direction = Vec2::Y;
                    heading.0 = std::f32::consts::FRAC_PI_2;
                }
            }
            Some(TrafficLane::Westbound) => {
                let pos = ground_pos.0;
                if pos.y > curve_center.y {
                    // Straight vertical segment heading South into the curve
                    let next_y = pos.y - car.speed * dt;
                    if next_y > curve_center.y {
                        ground_pos.y = next_y;
                        ground_pos.x = curve_start_x + r_west;
                        car.direction = -Vec2::Y;
                        heading.0 = -std::f32::consts::FRAC_PI_2;
                    } else {
                        // Transitions into curve
                        let excess_s = curve_center.y - next_y;
                        let theta = std::f32::consts::FRAC_PI_2 - (excess_s / r_west);
                        ground_pos.x = curve_start_x + r_west * theta.sin();
                        ground_pos.y = curve_center.y - r_west * theta.cos();
                        car.direction = Vec2::new(-theta.cos(), -theta.sin());
                        heading.0 = theta - std::f32::consts::PI;
                    }
                } else if pos.x > curve_start_x {
                    // On the curve moving towards theta = 0
                    let current_theta = (pos.x - curve_start_x).atan2(curve_center.y - pos.y);
                    let d_theta = (car.speed / r_west) * dt;
                    let next_theta = current_theta - d_theta;

                    if next_theta > 0.0 {
                        ground_pos.x = curve_start_x + r_west * next_theta.sin();
                        ground_pos.y = curve_center.y - r_west * next_theta.cos();
                        car.direction = Vec2::new(-next_theta.cos(), -next_theta.sin());
                        heading.0 = next_theta - std::f32::consts::PI;
                    } else {
                        // Exits curve to the West
                        let excess_s = (-next_theta) * r_west;
                        ground_pos.x = curve_start_x - excess_s;
                        ground_pos.y = west_lane_y;
                        car.direction = -Vec2::X;
                        heading.0 = std::f32::consts::PI;
                    }
                } else {
                    // Straight horizontal segment heading West
                    ground_pos.x -= car.speed * dt;
                    ground_pos.y = west_lane_y;
                    car.direction = -Vec2::X;
                    heading.0 = std::f32::consts::PI;
                }
            }
            Some(TrafficLane::Southbound) => {
                ground_pos.y -= car.speed * dt;
                ground_pos.x = south_lane_x;
                car.direction = -Vec2::Y;
                heading.0 = -std::f32::consts::FRAC_PI_2;
            }
            Some(TrafficLane::Northbound) => {
                ground_pos.y += car.speed * dt;
                ground_pos.x = north_lane_x;
                car.direction = Vec2::Y;
                heading.0 = std::f32::consts::FRAC_PI_2;
            }
            None => {
                let delta = car.direction * car.speed * dt;
                ground_pos.x += delta.x;
                ground_pos.y += delta.y;
            }
        }
    }
}

/// System to despawn cars that have completely exited the opposite playfield boundary
pub fn car_cleanup(mut commands: Commands, query: Query<(Entity, &Car, &GroundPos)>) {
    for (entity, car, ground_pos) in &query {
        let pos = ground_pos.0;
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
        assert_eq!(car_h.length, 48.0);
        assert_eq!(car_h.width, 26.0);
        assert_eq!(car_h.dimensions(), Vec2::new(48.0, 26.0));
        assert_eq!(car_h.half_extents(), Vec2::new(24.0, 13.0));

        let car_v = Car::new(240.0, Vec2::new(0.0, -1.0), 48.0, 26.0);
        assert_eq!(car_v.dimensions(), Vec2::new(48.0, 26.0));
        assert_eq!(car_v.half_extents(), Vec2::new(24.0, 13.0));
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
            Vec2::new(CAR_LENGTH, CAR_WIDTH)
        );
        assert_eq!(
            TrafficLane::Northbound.car_dimensions(),
            Vec2::new(CAR_LENGTH, CAR_WIDTH)
        );
    }

    #[test]
    fn test_traffic_lane_spawn_positions() {
        let geom = RoadGeometry::default();
        let east_pos = TrafficLane::Eastbound.spawn_position(&geom);
        assert_eq!(east_pos.x, -HALF_WORLD_WIDTH - SPAWN_MARGIN);
        assert_eq!(east_pos.y, geom.east_lane_y());
        assert_eq!(east_pos.z, Z_CAR);

        let west_pos = TrafficLane::Westbound.spawn_position(&geom);
        assert_eq!(west_pos.x, geom.curve_west_lane_x());
        assert_eq!(west_pos.y, HALF_WORLD_HEIGHT + SPAWN_MARGIN);
        assert_eq!(west_pos.z, Z_CAR);

        let south_pos = TrafficLane::Southbound.spawn_position(&geom);
        assert_eq!(south_pos.x, geom.south_lane_x());
        assert_eq!(south_pos.y, HALF_WORLD_HEIGHT + SPAWN_MARGIN);
        assert_eq!(south_pos.z, Z_CAR);

        let north_pos = TrafficLane::Northbound.spawn_position(&geom);
        assert_eq!(north_pos.x, geom.north_lane_x());
        assert_eq!(north_pos.y, -HALF_WORLD_HEIGHT - SPAWN_MARGIN);
        assert_eq!(north_pos.z, Z_CAR);
    }

    #[test]
    fn test_is_car_out_of_bounds() {
        let despawn_x = HALF_WORLD_WIDTH + DESPAWN_MARGIN;
        let despawn_y = HALF_WORLD_HEIGHT + DESPAWN_MARGIN;

        // Eastbound (heading +X)
        assert!(!is_car_out_of_bounds(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0)
        ));
        assert!(!is_car_out_of_bounds(
            Vec2::new(despawn_x, 0.0),
            Vec2::new(1.0, 0.0)
        ));
        assert!(is_car_out_of_bounds(
            Vec2::new(despawn_x + 0.1, 0.0),
            Vec2::new(1.0, 0.0)
        ));

        // Westbound (heading -X)
        assert!(!is_car_out_of_bounds(
            Vec2::new(-despawn_x, 0.0),
            Vec2::new(-1.0, 0.0)
        ));
        assert!(is_car_out_of_bounds(
            Vec2::new(-despawn_x - 0.1, 0.0),
            Vec2::new(-1.0, 0.0)
        ));

        // Northbound (heading +Y)
        assert!(!is_car_out_of_bounds(
            Vec2::new(0.0, despawn_y),
            Vec2::new(0.0, 1.0)
        ));
        assert!(is_car_out_of_bounds(
            Vec2::new(0.0, despawn_y + 0.1),
            Vec2::new(0.0, 1.0)
        ));

        // Southbound (heading -Y)
        assert!(!is_car_out_of_bounds(
            Vec2::new(0.0, -despawn_y),
            Vec2::new(0.0, -1.0)
        ));
        assert!(is_car_out_of_bounds(
            Vec2::new(0.0, -despawn_y - 0.1),
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
                GroundPos(Vec2::new(0.0, -30.0)),
                Heading::ZERO,
            ))
            .id();

        let car_s = app
            .world_mut()
            .spawn((
                Car::new(100.0, Vec2::new(0.0, -1.0), 26.0, 48.0),
                GroundPos(Vec2::new(-30.0, 100.0)),
                Heading::ZERO,
            ))
            .id();

        app.add_systems(Update, car_movement);

        // Advance 0.5s
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(500));

        app.update();

        let pos_e = app.world().get::<GroundPos>(car_e).unwrap();
        assert_eq!(pos_e.x, 100.0); // 200 * 0.5
        assert_eq!(pos_e.y, -30.0);

        let pos_s = app.world().get::<GroundPos>(car_s).unwrap();
        assert_eq!(pos_s.x, -30.0);
        assert_eq!(pos_s.y, 50.0); // 100 - 100 * 0.5
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
                GroundPos(Vec2::new(100.0, -30.0)),
            ))
            .id();

        // Exited car (+X eastbound past despawn)
        let despawn_x = HALF_WORLD_WIDTH + DESPAWN_MARGIN;
        let car_dead_e = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::new(1.0, 0.0), 48.0, 26.0),
                GroundPos(Vec2::new(despawn_x + 1.0, -30.0)),
            ))
            .id();

        // Exited car (-Y southbound past despawn)
        let despawn_y = HALF_WORLD_HEIGHT + DESPAWN_MARGIN;
        let car_dead_s = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::new(0.0, -1.0), 48.0, 26.0),
                GroundPos(Vec2::new(-30.0, -despawn_y - 5.0)),
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
        assert_eq!(car_ref.length, CAR_LENGTH);
        assert_eq!(car_ref.width, CAR_WIDTH);

        let ground_pos = app
            .world()
            .get::<GroundPos>(car_id)
            .expect("GroundPos should exist");
        assert_eq!(ground_pos.x, -HALF_WORLD_WIDTH - SPAWN_MARGIN);
        assert_eq!(ground_pos.y, geom.east_lane_y());

        let heading = app
            .world()
            .get::<Heading>(car_id)
            .expect("Heading should exist");
        assert_eq!(heading.0, 0.0);
        assert!(app.world().get::<Mesh2d>(car_id).is_some());
        assert!(
            app.world()
                .get::<MeshMaterial2d<ColorMaterial>>(car_id)
                .is_some()
        );
    }

    #[test]
    fn test_spawn_car_in_each_lane_heading_and_dimensions() {
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

        for lane in TrafficLane::ALL {
            let car_id = {
                let mut system_state = SpawnCarState::new(app.world_mut());
                let (mut cmd, mut meshes, mut materials) =
                    system_state.get_mut(app.world_mut()).unwrap();
                let id = spawn_car_in_lane(&mut cmd, &mut meshes, &mut materials, &geom, lane, 0);
                system_state.apply(app.world_mut());
                id
            };

            let car = app.world().get::<Car>(car_id).unwrap();
            assert_eq!(car.length, CAR_LENGTH);
            assert_eq!(car.width, CAR_WIDTH);
            assert_eq!(car.dimensions(), Vec2::new(CAR_LENGTH, CAR_WIDTH));
            assert_eq!(
                car.half_extents(),
                Vec2::new(CAR_LENGTH / 2.0, CAR_WIDTH / 2.0)
            );

            let heading = app.world().get::<Heading>(car_id).unwrap();
            assert_eq!(heading.0, lane.spawn_heading().0);

            match lane {
                TrafficLane::Eastbound => assert_eq!(heading.0, 0.0),
                TrafficLane::Westbound => assert_eq!(heading.0, -std::f32::consts::FRAC_PI_2),
                TrafficLane::Southbound => assert_eq!(heading.0, -std::f32::consts::FRAC_PI_2),
                TrafficLane::Northbound => assert_eq!(heading.0, std::f32::consts::FRAC_PI_2),
            }
        }
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

        let mut observed_lanes = std::collections::HashSet::new();
        let mut max_active_cars = 0;

        for _ in 0..total_frames {
            app.update();

            let mut car_query = app.world_mut().query::<(&Car, &GroundPos)>();
            let car_count = car_query.iter(app.world()).count();
            if car_count > max_active_cars {
                max_active_cars = car_count;
            }

            for (car, pos) in car_query.iter(app.world()) {
                assert!((car.direction.length() - 1.0).abs() < 1e-4);
                assert_eq!(car.speed, CAR_SPEED);
                if let Some(lane) = car.lane {
                    observed_lanes.insert(lane);
                }

                let pos = pos.0;

                assert!(!is_car_out_of_bounds(pos, car.direction));
            }
        }

        assert_eq!(observed_lanes.len(), 4, "All 4 lanes should have spawned");
        assert!(
            max_active_cars >= 2,
            "Cars should have been active concurrently"
        );
        assert!(
            max_active_cars <= 12,
            "Active cars should be bounded, got {}",
            max_active_cars
        );

        // Pause spawning and run until all existing cars despawn to prove zero entity leakage
        {
            let mut timer = app.world_mut().resource_mut::<SpawnTimer>();
            timer.set_duration(std::time::Duration::from_secs(1000));
            timer.reset();
        }

        for _ in 0..200 {
            app.update();
        }

        let remaining_cars = app.world_mut().query::<&Car>().iter(app.world()).count();
        assert_eq!(
            remaining_cars, 0,
            "All cars should have despawned cleanly without entity leakage"
        );
    }

    #[test]
    fn test_eastbound_car_follows_curve() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, car_movement);

        // Spawn eastbound car just before the curve at x = 150
        let car_id = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::X, 48.0, 26.0).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(150.0, -30.0)),
                Heading::ZERO,
            ))
            .id();

        // Advance 0.1s -> should enter curve (x > 160, y starts turning upward)
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(100));
        app.update();

        let mut query = app.world_mut().query::<(&Car, &GroundPos)>();
        let (car, pos) = query.get(app.world(), car_id).unwrap();
        assert!(pos.x >= 160.0);
        assert!(pos.y > -30.0);
        assert!(car.direction.y > 0.0);
    }

    #[test]
    fn test_westbound_car_follows_curve() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, car_movement);

        // Spawn westbound car above curve at y = 510, x = 630
        let car_id = app
            .world_mut()
            .spawn((
                Car::new(200.0, -Vec2::Y, 48.0, 26.0).with_lane(TrafficLane::Westbound),
                GroundPos(Vec2::new(630.0, 510.0)),
                Heading::new(-std::f32::consts::FRAC_PI_2),
            ))
            .id();

        // Advance 0.1s -> enters curve (y < 500, moving towards x < 630)
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(100));
        app.update();

        let mut query = app.world_mut().query::<(&Car, &GroundPos)>();
        let (car, pos) = query.get(app.world(), car_id).unwrap();
        assert!(pos.y <= 500.0);
        assert!(pos.x < 630.0);
        assert!(car.direction.x < 0.0);
    }

    #[test]
    fn test_eastbound_and_westbound_complete_curve_traversal() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<RoadGeometry>()
            .add_systems(Update, car_movement);

        let geom = RoadGeometry::default();

        // Eastbound car starts at x = 0, y = -30
        let car_e = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, 48.0, 26.0).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(0.0, geom.east_lane_y())),
                Heading::ZERO,
            ))
            .id();

        // Westbound car starts at x = 630, y = 560
        let car_w = app
            .world_mut()
            .spawn((
                Car::new(240.0, -Vec2::Y, 48.0, 26.0).with_lane(TrafficLane::Westbound),
                GroundPos(Vec2::new(geom.curve_west_lane_x(), 560.0)),
                Heading::new(-std::f32::consts::FRAC_PI_2),
            ))
            .id();

        // Step forward through time (5 seconds)
        let dt = 0.05;
        let steps = 100;
        for _ in 0..steps {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(dt));
            app.update();
        }

        let mut query = app.world_mut().query::<(&Car, &GroundPos)>();

        // Eastbound car has traversed curve and is now moving northbound at x = 690, y > 500
        let (car_e_ref, pos_e) = query.get(app.world(), car_e).unwrap();
        assert_eq!(pos_e.x, geom.curve_east_lane_x());
        assert!(pos_e.y >= 500.0);
        assert_eq!(car_e_ref.direction, Vec2::Y);

        // Westbound car has traversed curve and is now moving westbound at x < 160, y = 30
        let (car_w_ref, pos_w) = query.get(app.world(), car_w).unwrap();
        assert!(pos_w.x <= geom.curve_start_x);
        assert_eq!(pos_w.y, geom.west_lane_y());
        assert_eq!(car_w_ref.direction, -Vec2::X);
    }
}
