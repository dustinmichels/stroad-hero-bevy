use std::ops::{Deref, DerefMut};

use bevy::prelude::*;

use crate::constants::*;
use crate::road::RoadGeometry;
use crate::sim::{GroundPos, Heading};
use crate::state::{AppState, GameSet};
/// Default vehicle colors for visual variety
pub const CAR_COLORS: [Color; 5] = [
    Color::srgb(0.85, 0.22, 0.22), // Crimson Red
    Color::srgb(0.22, 0.52, 0.88), // Royal Blue
    Color::srgb(0.95, 0.62, 0.12), // Amber Orange
    Color::srgb(0.92, 0.78, 0.16), // Taxi Yellow
    Color::srgb(0.58, 0.28, 0.78), // Purple
];

/// Color for braking cars (distinct bright brake-light red)
pub const COLOR_CAR_BRAKING: Color = Color::srgb(0.95, 0.10, 0.10);
/// Color for crashed cars (distinct crumpled dark charcoal/slate)
pub const COLOR_CAR_CRASHED: Color = Color::srgb(0.32, 0.32, 0.35);

/// Driving-state visual classification for vehicles.
///
/// Simulation systems determine vehicle motion and collision states;
/// rendering views consume this classification to select shared materials
/// without housing simulation decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum CarVisualState {
    /// Cruising or accelerating normally.
    Normal,
    /// Actively decelerating, emergency braking, or stationary holding brakes.
    Braking,
    /// Involved in a collision / wreck awaiting cleanup.
    Crashed,
}

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
    pub fn spawn_position(&self, road_geom: &RoadGeometry) -> Vec2 {
        let spawn_x_dist = road_geom.world_width / 2.0 + SPAWN_MARGIN;
        let spawn_y_dist = road_geom.world_height / 2.0 + SPAWN_MARGIN;

        match self {
            TrafficLane::Eastbound => Vec2::new(-spawn_x_dist, road_geom.east_lane_y()),
            TrafficLane::Westbound => Vec2::new(road_geom.curve_west_lane_x(), spawn_y_dist),
            TrafficLane::Southbound => Vec2::new(road_geom.south_lane_x(), spawn_y_dist),
            TrafficLane::Northbound => Vec2::new(road_geom.north_lane_x(), -spawn_y_dist),
        }
    }

    /// Progress along this lane from its spawn origin in the travel direction.
    pub fn progress(&self, position: Vec2, road_geom: &RoadGeometry) -> f32 {
        lane_progress(*self, position, road_geom)
    }
    /// Progress coordinate where a car center comes to rest before an intersection stop line,
    /// stopped one half-car-length plus a buffer before `intersection_bounds`.
    pub fn stop_line_progress(&self, road_geom: &RoadGeometry) -> f32 {
        stop_line_progress(*self, road_geom)
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

    /// Zero-based lane index for dense array lookups
    #[inline]
    pub const fn index(&self) -> usize {
        match self {
            TrafficLane::Eastbound => 0,
            TrafficLane::Westbound => 1,
            TrafficLane::Southbound => 2,
            TrafficLane::Northbound => 3,
        }
    }
}

/// Pure function to calculate progress along a traffic lane from its spawn origin
/// and increasing monotonically in the travel direction.
pub fn lane_progress(lane: TrafficLane, position: Vec2, road_geom: &RoadGeometry) -> f32 {
    let half_w = road_geom.world_width / 2.0;
    let half_h = road_geom.world_height / 2.0;
    let spawn_x_dist = half_w + SPAWN_MARGIN;
    let spawn_y_dist = half_h + SPAWN_MARGIN;

    match lane {
        TrafficLane::Eastbound => {
            let x_spawn = -spawn_x_dist;
            let s1 = road_geom.curve_start_x - x_spawn;
            let r_east = road_geom.curve_east_lane_radius();
            let s2 = r_east * std::f32::consts::FRAC_PI_2;

            if position.y >= road_geom.curve_center.y {
                s1 + s2 + (position.y - road_geom.curve_center.y)
            } else if position.x <= road_geom.curve_start_x {
                position.x - x_spawn
            } else {
                let theta = (position.x - road_geom.curve_start_x)
                    .atan2(road_geom.curve_center.y - position.y)
                    .clamp(0.0, std::f32::consts::FRAC_PI_2);
                s1 + r_east * theta
            }
        }
        TrafficLane::Westbound => {
            let y_spawn = spawn_y_dist;
            let s1 = y_spawn - road_geom.curve_center.y;
            let r_west = road_geom.curve_west_lane_radius();
            let s2 = r_west * std::f32::consts::FRAC_PI_2;

            if position.y >= road_geom.curve_center.y {
                y_spawn - position.y
            } else if position.x <= road_geom.curve_start_x {
                s1 + s2 + (road_geom.curve_start_x - position.x)
            } else {
                let theta = (position.x - road_geom.curve_start_x)
                    .atan2(road_geom.curve_center.y - position.y)
                    .clamp(0.0, std::f32::consts::FRAC_PI_2);
                s1 + r_west * (std::f32::consts::FRAC_PI_2 - theta)
            }
        }
        TrafficLane::Southbound => spawn_y_dist - position.y,
        TrafficLane::Northbound => position.y - (-spawn_y_dist),
    }
}

/// Calculates the progress coordinate for a car center stopped one half-car-length
/// plus a small buffer before `intersection_bounds`.
pub fn stop_line_progress(lane: TrafficLane, road_geom: &RoadGeometry) -> f32 {
    let stop_pos = road_geom.stop_line_position(lane);
    let line_progress = lane_progress(lane, stop_pos, road_geom);
    line_progress - (CAR_LENGTH / 2.0 + STOP_LINE_BUFFER)
}

/// Calculates the front bumper progress along the route given center progress and car length.
/// Route progress increases in the travel direction, so the front bumper is ahead of center.
#[inline]
pub fn front_progress(center_progress: f32, car_length: f32) -> f32 {
    center_progress + car_length / 2.0
}

/// Calculates the rear bumper progress along the route given center progress and car length.
/// Route progress increases in the travel direction, so the rear bumper is behind center.
#[inline]
pub fn rear_progress(center_progress: f32, car_length: f32) -> f32 {
    center_progress - car_length / 2.0
}

/// Computes the bumper-to-bumper gap between a leader car and a follower car in the same lane.
/// A positive value represents the free distance between the leader's rear bumper and the follower's front bumper.
/// A zero or negative value indicates contact or overlap.
#[inline]
pub fn bumper_gap(leader_rear_progress: f32, follower_front_progress: f32) -> f32 {
    leader_rear_progress - follower_front_progress
}

/// Component representing an active vehicle in the traffic system
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Car {
    /// Design / cruise speed (units per second)
    pub speed: f32,
    /// Fixed or current unit direction vector
    pub direction: Vec2,
    /// Bounding box length along heading
    pub length: f32,
    /// Bounding box width perpendicular to heading
    pub width: f32,
    /// Assigned traffic lane, if spawned via traffic system
    pub lane: Option<TrafficLane>,
    /// Color index into `CAR_COLORS`
    pub color_index: usize,
    /// Whether this vehicle committed to crossing during a yellow signal
    pub committed_to_cross: bool,
}

/// Instantaneous vehicle motion state and driver speed limits
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
pub struct CarMotion {
    /// Instantaneous travel speed along route (units per second)
    pub current_speed: f32,
    /// Target speed the vehicle is actively accelerating or braking towards
    pub desired_speed: f32,
    /// Speed limit currently perceived and acted upon after reaction delay
    pub perceived_speed_limit: f32,
    /// Pending speed limit currently queued in driver reaction delay
    pub pending_speed_limit: f32,
    /// Remaining driver reaction delay before pending limit takes effect (seconds)
    pub reaction_remaining: f32,
    /// Whether this vehicle committed to crossing during a yellow signal
    pub committed_to_cross: bool,
    /// Free bumper gap to the nearest vehicle ahead in the same lane (None if leader)
    pub leader_gap: Option<f32>,
    /// Whether emergency braking is currently engaged (reacted safe speed below comfortable reach)
    pub emergency_braking: bool,
    /// Whether the driver has reacted and is currently following a tightened limit profile
    pub following_limit: bool,
    /// Whether this vehicle has begun stopping for an upcoming signal restriction
    pub stopping_for_signal: bool,
    /// Design / cruise speed
    pub cruise_speed: f32,
}

impl CarMotion {
    pub fn new(cruise_speed: f32) -> Self {
        Self {
            current_speed: cruise_speed,
            desired_speed: cruise_speed,
            perceived_speed_limit: cruise_speed,
            pending_speed_limit: cruise_speed,
            reaction_remaining: 0.0,
            committed_to_cross: false,
            stopping_for_signal: false,
            leader_gap: None,
            emergency_braking: false,
            following_limit: false,
            cruise_speed,
        }
    }

    pub fn with_speeds(mut self, current: f32, desired: f32) -> Self {
        self.current_speed = current;
        self.desired_speed = desired;
        self
    }

    pub fn with_limits(mut self, perceived: f32, pending: f32, reaction_remaining: f32) -> Self {
        self.perceived_speed_limit = perceived;
        self.pending_speed_limit = pending;
        self.reaction_remaining = reaction_remaining;
        self
    }

    pub fn with_committed_to_cross(mut self, committed: bool) -> Self {
        self.committed_to_cross = committed;
        self
    }

    pub fn with_emergency_braking(mut self, emergency: bool) -> Self {
        self.emergency_braking = emergency;
        self
    }
    /// Sets all motion speeds and limits to zero, disengaging emergency braking and pending limits.
    pub fn stop_motion(&mut self) {
        self.current_speed = 0.0;
        self.desired_speed = 0.0;
        self.perceived_speed_limit = 0.0;
        self.pending_speed_limit = 0.0;
        self.reaction_remaining = 0.0;
        self.emergency_braking = false;
        self.following_limit = false;
        self.stopping_for_signal = false;
        self.cruise_speed = 0.0;
    }

    /// Returns true if the vehicle is actively braking or holding brakes at a stop.
    ///
    /// Evaluates to true if emergency braking is engaged, if desired speed is lower than
    /// current speed (actively slowing down), or if the car is stationary with zero desired
    /// speed while having a non-zero cruise speed (holding brakes at a stop light or behind a leader).
    #[inline]
    pub fn is_braking(&self) -> bool {
        self.emergency_braking
            || (self.desired_speed < self.current_speed - 0.01)
            || (self.current_speed <= 0.01 && self.desired_speed <= 0.01 && self.cruise_speed > 0.0)
    }

    /// Computes the visual driving state for this motion, prioritizing crash state over braking.
    #[inline]
    pub fn visual_state(&self, is_crashed: bool) -> CarVisualState {
        if is_crashed {
            CarVisualState::Crashed
        } else if self.is_braking() {
            CarVisualState::Braking
        } else {
            CarVisualState::Normal
        }
    }

    /// Updates driver reaction and speed limits based on newly observed limit and elapsed time `dt`.
    ///
    /// Reaction delay is modeled only when a newly observed limit is stricter than `perceived_speed_limit`:
    /// - Holds the old `perceived_speed_limit` while `reaction_remaining > 0.0`.
    /// - Retains the strictest pending limit without restarting the reaction timer.
    /// - Applies the pending limit once `reaction_remaining <= 0.0`.
    /// - Relaxed limits apply immediately, allowing queues to move promptly on green.
    pub fn update_driver_reaction(&mut self, observed_limit: f32, dt: f32) {
        let observed = if observed_limit.is_finite() {
            observed_limit.max(0.0)
        } else {
            0.0
        };

        if observed >= self.perceived_speed_limit {
            // Relaxed limit: applies immediately
            self.perceived_speed_limit = observed;
            self.pending_speed_limit = observed;
            self.reaction_remaining = 0.0;
            if observed >= self.cruise_speed {
                self.following_limit = false;
            }
        } else if self.following_limit {
            // Already reacted and actively following a tightened limit:
            // Apply later tightening immediately without re-arming timer
            self.perceived_speed_limit = observed;
            self.pending_speed_limit = observed;
            self.reaction_remaining = 0.0;
        } else {
            let max_normal_drop = CAR_COMFORTABLE_DECELERATION * dt + 1.0;
            if (self.perceived_speed_limit - observed) <= max_normal_drop {
                // Small drop: enter following_limit immediately without reaction delay
                self.perceived_speed_limit = observed;
                self.pending_speed_limit = observed;
                self.reaction_remaining = 0.0;
                self.following_limit = true;
            } else {
                if self.reaction_remaining <= 0.0 {
                    // Start reaction delay for newly observed stricter limit
                    self.reaction_remaining = DRIVER_REACTION_SECS;
                    self.pending_speed_limit = observed;
                } else {
                    // Timer already running: retain strictest pending limit without restarting timer
                    self.pending_speed_limit = self.pending_speed_limit.min(observed);
                }

                if dt > 0.0 {
                    self.reaction_remaining = (self.reaction_remaining - dt).max(0.0);
                }

                if self.reaction_remaining <= 0.0 {
                    self.perceived_speed_limit = self.pending_speed_limit;
                    self.following_limit = true;
                }
            }
        }
        self.desired_speed = self.perceived_speed_limit;
        let comfortable_floor = self.current_speed - CAR_COMFORTABLE_DECELERATION * dt;
        self.emergency_braking =
            self.reaction_remaining <= 0.0 && self.desired_speed < comfortable_floor;
    }
    #[inline]
    pub fn step(&mut self, dt: f32) -> f32 {
        let decel = if self.emergency_braking {
            CAR_EMERGENCY_DECELERATION
        } else {
            CAR_COMFORTABLE_DECELERATION
        };
        self.step_with_decel(dt, decel)
    }

    /// Advances `current_speed` toward `desired_speed` by `dt` seconds using the given deceleration limit.
    #[inline]
    pub fn step_with_decel(&mut self, dt: f32, deceleration: f32) -> f32 {
        self.current_speed = step_speed(
            self.current_speed,
            self.desired_speed,
            CAR_ACCELERATION,
            deceleration,
            dt,
        );
        self.current_speed
    }
}

/// Component representing a vehicle in a crashed state.
/// Holds a countdown timer until the crashed car is despawned.
#[derive(Component, Debug, Clone, Reflect)]
pub struct Crashed {
    pub timer: Timer,
}

impl Crashed {
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(CRASH_CLEANUP_SECS, TimerMode::Once),
        }
    }

    pub fn with_duration(secs: f32) -> Self {
        Self {
            timer: Timer::from_seconds(secs, TimerMode::Once),
        }
    }
}

impl Default for Crashed {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CarMotion {
    fn default() -> Self {
        Self::new(CAR_SPEED)
    }
}

/// Pure speed-step helper that clamps acceleration/deceleration by `dt`,
/// never produces a negative speed, and never overshoots the desired speed.
pub fn step_speed(
    current_speed: f32,
    desired_speed: f32,
    acceleration: f32,
    deceleration: f32,
    dt: f32,
) -> f32 {
    if dt <= 0.0 {
        return current_speed.max(0.0);
    }

    let current = current_speed.max(0.0);
    let target = desired_speed.max(0.0);

    if target > current {
        let max_increase = acceleration.max(0.0) * dt;
        (current + max_increase).min(target)
    } else if target < current {
        let max_decrease = deceleration.max(0.0) * dt;
        (current - max_decrease).max(target).max(0.0)
    } else {
        current
    }
}

impl Car {
    pub fn new(speed: f32, direction: Vec2, length: f32, width: f32) -> Self {
        Self {
            speed,
            direction: direction.normalize_or_zero(),
            length,
            width,
            lane: None,
            color_index: 0,
            committed_to_cross: false,
        }
    }

    pub fn with_color_index(mut self, color_index: usize) -> Self {
        self.color_index = color_index;
        self
    }

    pub fn with_lane(mut self, lane: TrafficLane) -> Self {
        self.lane = Some(lane);
        self
    }
    pub fn with_committed_to_cross(mut self, committed: bool) -> Self {
        self.committed_to_cross = committed;
        self
    }

    /// Updates yellow crossing commitment given center progress and stop line progress.
    /// Clears commitment after the rear bumper passes the stop line.
    #[inline]
    pub fn update_crossing_commitment(&mut self, center_progress: f32, stop_line_coord: f32) {
        if self.rear_progress(center_progress) >= stop_line_coord {
            self.committed_to_cross = false;
        }
    }

    pub fn dimensions(&self) -> Vec2 {
        Vec2::new(self.length, self.width)
    }

    pub fn half_extents(&self) -> Vec2 {
        Vec2::new(self.length / 2.0, self.width / 2.0)
    }
    /// Front bumper progress along the route given the car's center progress
    #[inline]
    pub fn front_progress(&self, center_progress: f32) -> f32 {
        front_progress(center_progress, self.length)
    }

    /// Rear bumper progress along the route given the car's center progress
    #[inline]
    pub fn rear_progress(&self, center_progress: f32) -> f32 {
        rear_progress(center_progress, self.length)
    }
}

/// Resource controlling periodic vehicle generation across intersection lanes
#[derive(Resource, Debug, Clone)]
pub struct SpawnTimer {
    pub timer: Timer,
    pub next_lane_index: usize,
    pub deferred: bool,
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
            deferred: false,
        }
    }

    /// Return the lane that will spawn next without advancing the internal lane index
    pub fn current_lane(&self) -> TrafficLane {
        TrafficLane::ALL[self.next_lane_index % TrafficLane::ALL.len()]
    }

    /// Reset the spawn timer and clear any deferred spawn state
    pub fn reset(&mut self) {
        self.timer.reset();
        self.deferred = false;
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

/// State of a traffic signal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum TrafficLightState {
    #[default]
    Green,
    Yellow,
    Red,
}

impl TrafficLightState {
    pub const fn duration_secs(&self) -> f32 {
        match self {
            TrafficLightState::Green => SIGNAL_GREEN_SECS,
            TrafficLightState::Yellow => SIGNAL_YELLOW_SECS,
            TrafficLightState::Red => SIGNAL_RED_SECS,
        }
    }

    pub const fn next(&self) -> Self {
        match self {
            TrafficLightState::Green => TrafficLightState::Yellow,
            TrafficLightState::Yellow => TrafficLightState::Red,
            TrafficLightState::Red => TrafficLightState::Green,
        }
    }
}

/// Resource holding the current traffic signal state and repeating phase timer
#[derive(Resource, Debug, Clone, Reflect)]
pub struct TrafficSignal {
    pub state: TrafficLightState,
    pub timer: Timer,
}

impl Deref for TrafficSignal {
    type Target = Timer;

    fn deref(&self) -> &Self::Target {
        &self.timer
    }
}

impl DerefMut for TrafficSignal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.timer
    }
}

impl TrafficSignal {
    pub fn new(state: TrafficLightState, duration_secs: f32) -> Self {
        Self {
            state,
            timer: Timer::from_seconds(duration_secs, TimerMode::Repeating),
        }
    }
    /// Advance the traffic signal by `delta`, executing deterministic
    /// Green -> Yellow -> Red -> Green transitions and correctly rolling over leftover time.
    pub fn tick(&mut self, delta: std::time::Duration) {
        if delta.is_zero() {
            return;
        }

        let mut remaining = delta;
        while !remaining.is_zero() {
            let needed = self.timer.duration().saturating_sub(self.timer.elapsed());
            if needed.is_zero() || remaining >= needed {
                remaining = remaining.saturating_sub(needed);
                let next_state = self.state.next();
                let next_duration = std::time::Duration::from_secs_f32(next_state.duration_secs());
                self.state = next_state;
                self.timer.set_duration(next_duration);
                self.timer.reset();
            } else {
                self.timer.tick(remaining);
                break;
            }
        }
    }

    /// Resets the signal back to initial Green aspect and full green duration.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// System to advance the traffic signal timer and handle state transitions
pub fn tick_traffic_signal(time: Res<Time>, mut signal: ResMut<TrafficSignal>) {
    signal.tick(time.delta());
}

impl Default for TrafficSignal {
    fn default() -> Self {
        Self::new(TrafficLightState::Green, SIGNAL_GREEN_SECS)
    }
}

/// Decision made by a vehicle approaching a traffic signal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SignalApproachDecision {
    Proceed,
    Stop,
}

/// Result of evaluating approach behavior towards a traffic signal
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct SignalApproachResult {
    pub decision: SignalApproachDecision,
    pub committed_to_cross: bool,
}

/// Calculates stopping distance at given deceleration rate: d_stop = speed^2 / (2 * a)
#[inline]
pub fn stopping_distance(speed: f32, deceleration: f32) -> f32 {
    if deceleration <= 0.0 || speed <= 0.0 {
        0.0
    } else {
        (speed * speed) / (2.0 * deceleration)
    }
}

/// Evaluates vehicle approach behavior towards a traffic signal:
/// - Green: no signal speed restriction.
/// - Yellow: stop if the car can do so at comfortable deceleration before the line; otherwise commit to crossing.
/// - Red: stop before the line unless the car committed during yellow or its front bumper already crossed the line.
pub fn evaluate_signal_approach(
    signal: TrafficLightState,
    speed: f32,
    front_progress: f32,
    stop_line_progress: f32,
    committed_to_cross: bool,
    comfortable_deceleration: f32,
) -> SignalApproachResult {
    let crossed_stop_line = front_progress >= stop_line_progress;

    match signal {
        TrafficLightState::Green => SignalApproachResult {
            decision: SignalApproachDecision::Proceed,
            committed_to_cross,
        },
        TrafficLightState::Yellow => {
            if committed_to_cross || crossed_stop_line {
                SignalApproachResult {
                    decision: SignalApproachDecision::Proceed,
                    committed_to_cross: true,
                }
            } else {
                let dist_to_line = stop_line_progress - front_progress;
                let d_stop = stopping_distance(speed, comfortable_deceleration);
                if d_stop <= dist_to_line {
                    SignalApproachResult {
                        decision: SignalApproachDecision::Stop,
                        committed_to_cross: false,
                    }
                } else {
                    SignalApproachResult {
                        decision: SignalApproachDecision::Proceed,
                        committed_to_cross: true,
                    }
                }
            }
        }
        TrafficLightState::Red => {
            if committed_to_cross || crossed_stop_line {
                SignalApproachResult {
                    decision: SignalApproachDecision::Proceed,
                    committed_to_cross,
                }
            } else {
                SignalApproachResult {
                    decision: SignalApproachDecision::Stop,
                    committed_to_cross: false,
                }
            }
        }
    }
}

/// Updates or clears yellow crossing commitment for a vehicle based on its rear bumper progress.
/// Once the rear bumper passes the stop line, the commitment is cleared so future cycles or intersections
/// are not affected.
#[inline]
pub fn update_crossing_commitment(
    committed_to_cross: bool,
    rear_progress: f32,
    stop_line_progress: f32,
) -> bool {
    if rear_progress >= stop_line_progress {
        false
    } else {
        committed_to_cross
    }
}

/// System to update and clear signal crossing commitments for active cars
pub fn update_car_signal_commitments(
    signal: Res<TrafficSignal>,
    road_geom: Option<Res<RoadGeometry>>,
    mut query: Query<(&mut Car, &GroundPos)>,
) {
    let default_geom = RoadGeometry::default();
    let geom = road_geom.as_deref().unwrap_or(&default_geom);

    for (mut car, ground_pos) in &mut query {
        if let Some(lane) = car.lane {
            let center_progress = lane_progress(lane, ground_pos.0, geom);
            let stop_pos = geom.stop_line_position(lane);
            let stop_line = lane_progress(lane, stop_pos, geom);
            let rear_p = car.rear_progress(center_progress);
            let front_p = car.front_progress(center_progress);

            if rear_p >= stop_line {
                car.committed_to_cross = false;
            } else if signal.state == TrafficLightState::Yellow {
                let eval = evaluate_signal_approach(
                    signal.state,
                    car.speed,
                    front_p,
                    stop_line,
                    car.committed_to_cross,
                    CAR_COMFORTABLE_DECELERATION,
                );
                if eval.committed_to_cross {
                    car.committed_to_cross = true;
                }
            }
        }
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

    let car = Car::new(CAR_SPEED, initial_dir, dims.x, dims.y)
        .with_lane(lane)
        .with_color_index(color_index);
    let heading = lane.spawn_heading();

    commands
        .spawn((
            GroundPos(spawn_pos),
            heading,
            car,
            CarMotion::new(CAR_SPEED),
        ))
        .id()
}

/// Returns whether the specified lane's spawn footprint has enough clearance
/// (at least `CAR_MIN_FOLLOW_GAP`) from any existing vehicles in that lane.
pub fn is_spawn_footprint_clear<'a, I>(lane: TrafficLane, cars: I, road_geom: &RoadGeometry) -> bool
where
    I: IntoIterator<Item = (&'a Car, &'a GroundPos)>,
{
    let spawn_pos = lane.spawn_position(road_geom);
    let spawn_p = lane_progress(lane, spawn_pos, road_geom);
    let spawn_front_p = spawn_p + CAR_LENGTH / 2.0;

    for (car, ground_pos) in cars {
        if car.lane != Some(lane) {
            continue;
        }
        let car_p = lane_progress(lane, ground_pos.0, road_geom);
        let car_rear_p = car.rear_progress(car_p);
        let gap = bumper_gap(car_rear_p, spawn_front_p);
        if gap < CAR_MIN_FOLLOW_GAP {
            return false;
        }
    }

    true
}

/// System to periodically generate cars across intersection lanes.
/// Defers spawning into an occupied spawn footprint until enough bumper clearance exists.
pub fn spawn_cars(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<SpawnTimer>,
    road_geom: Res<RoadGeometry>,
    existing_cars: Query<(&Car, &GroundPos)>,
) {
    if spawn_timer.timer.tick(time.delta()).just_finished() {
        spawn_timer.deferred = true;
    }

    if spawn_timer.deferred {
        let lane = spawn_timer.current_lane();
        if is_spawn_footprint_clear(lane, existing_cars.iter(), &road_geom) {
            let color_idx = spawn_timer.next_lane_index;
            spawn_car_in_lane(&mut commands, &road_geom, lane, color_idx);
            spawn_timer.advance_lane();
            spawn_timer.deferred = false;
            spawn_timer.timer.reset();
        }
    }
}

/// Scratch entry for a car in a traffic lane, used during motion planning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneCarEntry {
    pub entity: Entity,
    pub progress: f32,
    pub front_progress: f32,
    pub rear_progress: f32,
    pub current_speed: f32,
    pub cruise_speed: f32,
    pub committed_to_cross: bool,
    pub stopping_for_signal: bool,
    pub crashed: bool,
}

/// Scratch buffers for grouping cars by lane in `plan_car_motion`.
/// Maintained across frames via Bevy `Local` to avoid per-frame allocations.
#[derive(Default)]
pub struct LaneCarScratch {
    pub lanes: [Vec<LaneCarEntry>; 4],
}

impl LaneCarScratch {
    pub fn clear(&mut self) {
        for lane in &mut self.lanes {
            lane.clear();
        }
    }
}

/// Helper to extract follower bumper gaps for an ordered lane slice:
/// returns `(follower_entity, gap_to_leader)`.
pub fn compute_lane_follower_gaps(lane_cars: &[LaneCarEntry]) -> Vec<(Entity, f32)> {
    let mut gaps = Vec::with_capacity(lane_cars.len().saturating_sub(1));
    for i in 1..lane_cars.len() {
        let leader = &lane_cars[i - 1];
        let follower = &lane_cars[i];
        gaps.push((
            follower.entity,
            bumper_gap(leader.rear_progress, follower.front_progress),
        ));
    }
    gaps
}

/// Pure helper to calculate safe stopping speed given available distance `d` and deceleration `a`:
/// v_safe = sqrt(2 * a * max(0, d)).
/// Non-positive or non-finite distances and decelerations clamp to 0.0.
#[inline]
pub fn safe_stopping_speed(distance: f32, deceleration: f32) -> f32 {
    safe_stopping_speed_with_reaction(distance, deceleration, 0.0)
}

/// Calculates safe stopping speed given available distance `d`, deceleration `a`, and reaction time `t_r`:
/// v_safe = a * (-t_r + sqrt(t_r^2 + 2*d/a)).
/// When t_r == 0, simplifies to sqrt(2 * a * d).
#[inline]
pub fn safe_stopping_speed_with_reaction(
    distance: f32,
    deceleration: f32,
    reaction_time: f32,
) -> f32 {
    if !distance.is_finite() || distance <= 0.0 || !deceleration.is_finite() || deceleration <= 0.0
    {
        0.0
    } else if reaction_time <= 0.0 || !reaction_time.is_finite() {
        (2.0 * deceleration * distance).sqrt()
    } else {
        let tr = reaction_time;
        let inner = tr * tr + (2.0 * distance) / deceleration;
        deceleration * (-tr + inner.sqrt()).max(0.0)
    }
}

/// Computes a car's observed speed limit as the most restrictive of:
/// - Cruise speed
/// - The speed that preserves CAR_MIN_FOLLOW_GAP behind the leader
/// - The speed that stops before a red/yellow stop line
///
/// Available distance `d` behind the leader is non-negative, and invalid/overlapping gaps
/// are clamped to zero.
#[inline]
pub fn compute_observed_speed_limit(
    cruise_speed: f32,
    leader_gap: Option<f32>,
    signal_limit: Option<f32>,
    deceleration: f32,
) -> f32 {
    let mut limit = if cruise_speed.is_finite() {
        cruise_speed.max(0.0)
    } else {
        0.0
    };

    if let Some(gap) = leader_gap {
        // Clamp invalid or overlapping bumper gaps to zero
        let clamped_gap = if gap.is_finite() && gap > 0.0 {
            gap
        } else {
            0.0
        };
        let available_d = (clamped_gap - CAR_MIN_FOLLOW_GAP).max(0.0);
        let follow_speed = safe_stopping_speed(available_d, deceleration);
        limit = limit.min(follow_speed);
    }

    if let Some(sig_limit) = signal_limit {
        if sig_limit.is_finite() {
            limit = limit.min(sig_limit.max(0.0));
        } else {
            limit = 0.0;
        }
    }

    limit.max(0.0)
}

/// Evaluates signal stopping speed limit and commitment for a car approaching an intersection.
#[allow(clippy::too_many_arguments)]
pub fn compute_signal_speed_limit(
    signal_state: TrafficLightState,
    center_progress: f32,
    front_progress: f32,
    rear_progress: f32,
    stop_line_mark: f32,
    stop_line_coord: f32,
    committed_to_cross: bool,
    stopping_for_signal: bool,
    current_speed: f32,
    deceleration: f32,
) -> (Option<f32>, bool, bool) {
    if rear_progress >= stop_line_mark {
        // Rear bumper cleared the stop line; commitment is reset and light does not restrict.
        return (None, false, false);
    }

    match signal_state {
        TrafficLightState::Green => (None, committed_to_cross, false),
        TrafficLightState::Yellow => {
            if committed_to_cross || front_progress >= stop_line_mark {
                (None, true, false)
            } else if stopping_for_signal {
                let d = (stop_line_coord - center_progress).max(0.0);
                (Some(safe_stopping_speed(d, deceleration)), false, true)
            } else {
                let dist_to_line = stop_line_mark - front_progress;
                let d_stop = stopping_distance(current_speed, deceleration);
                if d_stop <= dist_to_line {
                    let d = (stop_line_coord - center_progress).max(0.0);
                    (Some(safe_stopping_speed(d, deceleration)), false, true)
                } else {
                    (None, true, false)
                }
            }
        }
        TrafficLightState::Red => {
            if committed_to_cross || front_progress >= stop_line_mark {
                (None, true, false)
            } else {
                let d = (stop_line_coord - center_progress).max(0.0);
                (Some(safe_stopping_speed(d, deceleration)), false, true)
            }
        }
    }
}

/// Plans vehicle speeds before movement: groups cars by lane, orders by route progress,
/// and calculates safe following distances behind lead vehicles and stop lines.
pub fn plan_car_motion(
    time: Res<Time>,
    road_geom: Option<Res<RoadGeometry>>,
    signal: Option<Res<TrafficSignal>>,
    mut scratch: Local<LaneCarScratch>,
    mut query: Query<(Entity, &mut Car, &mut CarMotion, &GroundPos, Has<Crashed>)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    let default_geom = RoadGeometry::default();
    let geom = road_geom.as_deref().unwrap_or(&default_geom);
    let signal_state = signal
        .as_ref()
        .map(|s| s.state)
        .unwrap_or(TrafficLightState::Green);

    scratch.clear();

    // 1. Group live cars by TrafficLane
    for (entity, car, motion, ground_pos, is_crashed) in query.iter() {
        if let Some(lane) = car.lane {
            let center_p = lane_progress(lane, ground_pos.0, geom);
            scratch.lanes[lane.index()].push(LaneCarEntry {
                entity,
                progress: center_p,
                front_progress: car.front_progress(center_p),
                rear_progress: car.rear_progress(center_p),
                current_speed: motion.current_speed,
                cruise_speed: car.speed,
                committed_to_cross: car.committed_to_cross || motion.committed_to_cross,
                stopping_for_signal: motion.stopping_for_signal,
                crashed: is_crashed,
            });
        }
    }

    // 2. Order each group by lane_progress descending (leader with highest progress first)
    for lane_cars in &mut scratch.lanes {
        lane_cars.sort_by(|a, b| b.progress.total_cmp(&a.progress));
    }

    // 3. Compute limits and update CarMotion
    for lane_idx in 0..4 {
        let lane = TrafficLane::ALL[lane_idx];
        let stop_pos = geom.stop_line_position(lane);
        let stop_line_mark = lane_progress(lane, stop_pos, geom);
        let stop_line_coord = stop_line_progress(lane, geom);

        let lane_cars = &scratch.lanes[lane_idx];
        for (i, entry) in lane_cars.iter().enumerate() {
            let leader_gap = if i > 0 {
                let leader = &lane_cars[i - 1];
                Some(bumper_gap(leader.rear_progress, entry.front_progress))
            } else {
                None
            };

            let (signal_limit, committed, stopping) = compute_signal_speed_limit(
                signal_state,
                entry.progress,
                entry.front_progress,
                entry.rear_progress,
                stop_line_mark,
                stop_line_coord,
                entry.committed_to_cross,
                entry.stopping_for_signal,
                entry.current_speed,
                CAR_COMFORTABLE_DECELERATION,
            );

            let observed_limit = compute_observed_speed_limit(
                entry.cruise_speed,
                leader_gap,
                signal_limit,
                CAR_COMFORTABLE_DECELERATION,
            );

            if let Ok((_, mut car, mut motion, _, is_crashed)) = query.get_mut(entry.entity) {
                if is_crashed {
                    car.speed = 0.0;
                    motion.stop_motion();
                    continue;
                }
                motion.leader_gap = leader_gap;
                car.committed_to_cross = committed;
                motion.committed_to_cross = committed;
                motion.stopping_for_signal = stopping;
                motion.update_driver_reaction(observed_limit, dt);
            }
        }
    }
}

/// Advances each vehicle's `current_speed` toward `desired_speed` using finite acceleration / comfortable deceleration.
pub fn apply_car_acceleration(time: Res<Time>, mut query: Query<&mut CarMotion, Without<Crashed>>) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for mut motion in &mut query {
        if motion.current_speed != motion.desired_speed {
            motion.step(dt);
        }
    }
}

pub fn car_movement(
    time: Res<Time>,
    road_geom: Option<Res<RoadGeometry>>,
    mut query: Query<
        (&mut Car, Option<&CarMotion>, &mut GroundPos, &mut Heading),
        Without<Crashed>,
    >,
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

    for (mut car, motion, mut ground_pos, mut heading) in &mut query {
        let current_speed = motion.map_or(car.speed, |m| m.current_speed);
        if current_speed <= 0.0 {
            continue;
        }
        match car.lane {
            Some(TrafficLane::Eastbound) => {
                let pos = ground_pos.0;
                if pos.x < curve_start_x {
                    // Straight horizontal segment heading East
                    let next_x = pos.x + current_speed * dt;
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
                    let d_theta = (current_speed / r_east) * dt;
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
                    ground_pos.y += current_speed * dt;
                    ground_pos.x = curve_start_x + r_east;
                    car.direction = Vec2::Y;
                    heading.0 = std::f32::consts::FRAC_PI_2;
                }
            }
            Some(TrafficLane::Westbound) => {
                let pos = ground_pos.0;
                if pos.y > curve_center.y {
                    // Straight vertical segment heading South into the curve
                    let next_y = pos.y - current_speed * dt;
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
                    let d_theta = (current_speed / r_west) * dt;
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
                    ground_pos.x -= current_speed * dt;
                    ground_pos.y = west_lane_y;
                    car.direction = -Vec2::X;
                    heading.0 = std::f32::consts::PI;
                }
            }
            Some(TrafficLane::Southbound) => {
                ground_pos.y -= current_speed * dt;
                ground_pos.x = south_lane_x;
                car.direction = -Vec2::Y;
                heading.0 = -std::f32::consts::FRAC_PI_2;
            }
            Some(TrafficLane::Northbound) => {
                ground_pos.y += current_speed * dt;
                ground_pos.x = north_lane_x;
                car.direction = Vec2::Y;
                heading.0 = std::f32::consts::FRAC_PI_2;
            }
            None => {
                let delta = car.direction * current_speed * dt;
                ground_pos.x += delta.x;
                ground_pos.y += delta.y;
            }
        }
    }
}

/// Event emitted when two cars in the same lane collide rear-to-front
#[derive(Event, Debug, Clone, Copy, PartialEq)]
pub struct RearEndCrashEvent {
    pub leader: Entity,
    pub follower: Entity,
    pub lane: TrafficLane,
    pub gap: f32,
}

/// Detects rear-end collisions between adjacent cars in the same lane after movement.
/// A bumper gap <= 0 indicates a collision.
#[allow(clippy::type_complexity)]
pub fn detect_rear_end_crashes(
    mut commands: Commands,
    road_geom: Option<Res<RoadGeometry>>,
    mut scratch: Local<LaneCarScratch>,
    mut query: Query<(
        Entity,
        &mut Car,
        Option<&mut CarMotion>,
        &GroundPos,
        Has<Crashed>,
    )>,
) {
    let default_geom = RoadGeometry::default();
    let geom = road_geom.as_deref().unwrap_or(&default_geom);

    scratch.clear();

    for (entity, car, motion, ground_pos, is_crashed) in query.iter() {
        if let Some(lane) = car.lane {
            let center_p = lane_progress(lane, ground_pos.0, geom);
            scratch.lanes[lane.index()].push(LaneCarEntry {
                entity,
                progress: center_p,
                front_progress: car.front_progress(center_p),
                rear_progress: car.rear_progress(center_p),
                current_speed: motion.as_ref().map_or(car.speed, |m| m.current_speed),
                cruise_speed: car.speed,
                committed_to_cross: car.committed_to_cross
                    || motion.as_ref().is_some_and(|m| m.committed_to_cross),
                stopping_for_signal: motion.as_ref().is_some_and(|m| m.stopping_for_signal),
                crashed: is_crashed,
            });
        }
    }

    for lane_cars in &mut scratch.lanes {
        lane_cars.sort_by(|a, b| b.progress.total_cmp(&a.progress));
    }

    for lane_idx in 0..4 {
        let lane = TrafficLane::ALL[lane_idx];
        let lane_cars = &scratch.lanes[lane_idx];
        for i in 1..lane_cars.len() {
            let leader = &lane_cars[i - 1];
            let follower = &lane_cars[i];
            let gap = bumper_gap(leader.rear_progress, follower.front_progress);
            if gap <= 0.0 {
                if leader.crashed && follower.crashed {
                    continue;
                }

                commands.trigger(RearEndCrashEvent {
                    leader: leader.entity,
                    follower: follower.entity,
                    lane,
                    gap,
                });

                if !leader.crashed {
                    commands.entity(leader.entity).insert(Crashed::default());
                }
                if !follower.crashed {
                    commands.entity(follower.entity).insert(Crashed::default());
                }

                if let Ok((_, mut car, motion, _, _)) = query.get_mut(leader.entity) {
                    car.speed = 0.0;
                    if let Some(mut m) = motion {
                        m.stop_motion();
                    }
                }
                if let Ok((_, mut car, motion, _, _)) = query.get_mut(follower.entity) {
                    car.speed = 0.0;
                    if let Some(mut m) = motion {
                        m.stop_motion();
                    }
                }
            }
        }
    }
}

/// Advances the post-crash cleanup timer on crashed vehicles.
pub fn tick_crash_cleanup(time: Option<Res<Time>>, mut query: Query<&mut Crashed>) {
    let dt = time.as_ref().map(|t| t.delta()).unwrap_or_default();
    for mut crashed in &mut query {
        crashed.timer.tick(dt);
    }
}

/// System to despawn cars that have completely exited the opposite playfield boundary
/// or have exceeded their post-crash cleanup timer.
pub fn car_cleanup(
    mut commands: Commands,
    query: Query<(Entity, &Car, &GroundPos, Option<&Crashed>)>,
) {
    for (entity, car, ground_pos, maybe_crashed) in &query {
        if is_car_out_of_bounds(ground_pos.0, car.direction) {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(crashed) = maybe_crashed
            && crashed.timer.is_finished()
        {
            commands.entity(entity).despawn();
        }
    }
}

/// Plugin registering traffic resources and systems
pub struct TrafficPlugin;

impl Plugin for TrafficPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnTimer>()
            .init_resource::<TrafficSignal>()
            .add_systems(
                Update,
                (
                    (
                        tick_traffic_signal,
                        spawn_cars,
                        plan_car_motion,
                        apply_car_acceleration,
                        car_movement,
                    )
                        .chain()
                        .in_set(GameSet::Sim),
                    detect_rear_end_crashes.in_set(GameSet::Collision),
                    (tick_crash_cleanup, car_cleanup)
                        .chain()
                        .in_set(GameSet::Cleanup),
                )
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
    fn test_car_motion_component() {
        let motion = CarMotion::new(240.0);
        assert_eq!(motion.current_speed, 240.0);
        assert_eq!(motion.desired_speed, 240.0);
        assert_eq!(motion.perceived_speed_limit, 240.0);
        assert_eq!(motion.pending_speed_limit, 240.0);
        assert_eq!(motion.reaction_remaining, 0.0);
        assert!(!motion.committed_to_cross);

        let def = CarMotion::default();
        assert_eq!(def.current_speed, CAR_SPEED);

        let modified = motion
            .with_speeds(100.0, 150.0)
            .with_limits(150.0, 120.0, 0.35)
            .with_committed_to_cross(true);
        assert_eq!(modified.current_speed, 100.0);
        assert_eq!(modified.desired_speed, 150.0);
        assert_eq!(modified.perceived_speed_limit, 150.0);
        assert_eq!(modified.pending_speed_limit, 120.0);
        assert_eq!(modified.reaction_remaining, 0.35);
        assert!(modified.committed_to_cross);
    }

    #[test]
    fn test_step_speed_helper() {
        let accel = 160.0;
        let decel = 300.0;

        // Zero or negative dt returns current speed clamped to >= 0
        assert_eq!(step_speed(100.0, 200.0, accel, decel, 0.0), 100.0);
        assert_eq!(step_speed(100.0, 200.0, accel, decel, -1.0), 100.0);
        assert_eq!(step_speed(-10.0, 200.0, accel, decel, 0.0), 0.0);

        // Acceleration clamped by dt
        let dt = 0.5;
        assert_eq!(step_speed(0.0, 200.0, accel, decel, dt), 80.0);

        // Never overshoots desired speed when accelerating
        assert_eq!(step_speed(190.0, 200.0, accel, decel, dt), 200.0);

        // Deceleration clamped by dt
        assert_eq!(step_speed(200.0, 0.0, accel, decel, dt), 50.0);

        // Never overshoots desired speed when decelerating
        assert_eq!(step_speed(50.0, 0.0, accel, decel, dt), 0.0);

        // Never produces negative speed even if desired_speed < 0 or large decel
        assert_eq!(step_speed(50.0, -100.0, accel, decel, 1.0), 0.0);

        // CarMotion step methods
        let mut motion = CarMotion::new(0.0).with_speeds(0.0, 160.0);
        let new_speed = motion.step(0.5);
        assert_eq!(new_speed, 80.0);
        assert_eq!(motion.current_speed, 80.0);

        // Decel step
        motion.desired_speed = 0.0;
        let new_speed = motion.step_with_decel(0.1, 520.0);
        assert_eq!(new_speed, 28.0);
        assert_eq!(motion.current_speed, 28.0);
    }

    #[test]
    fn test_finite_acceleration_from_rest() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, (apply_car_acceleration, car_movement).chain());

        // Eastbound car starting from complete rest (current_speed = 0.0) with cruise desired_speed = 160.0
        let car_id = app
            .world_mut()
            .spawn((
                Car::new(160.0, Vec2::X, 48.0, 26.0).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(0.0, -30.0)),
                Heading::ZERO,
                CarMotion::new(160.0).with_speeds(0.0, 160.0),
            ))
            .id();

        // Advance 0.5s:
        // Acceleration = 160.0 units/s^2 -> current_speed becomes 0 + 160 * 0.5 = 80.0 units/s.
        // Movement = current_speed * dt = 80.0 * 0.5 = 40.0 units.
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(500));
        app.update();

        let motion = app.world().get::<CarMotion>(car_id).unwrap();
        let pos = app.world().get::<GroundPos>(car_id).unwrap();
        assert_eq!(motion.current_speed, 80.0);
        assert_eq!(pos.x, 40.0);

        // Advance another 0.5s:
        // current_speed reaches desired_speed: min(80 + 160 * 0.5, 160) = 160.0 units/s.
        // Movement = 160.0 * 0.5 = 80.0 units -> pos.x becomes 40.0 + 80.0 = 120.0.
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(500));
        app.update();

        let motion = app.world().get::<CarMotion>(car_id).unwrap();
        let pos = app.world().get::<GroundPos>(car_id).unwrap();
        assert_eq!(motion.current_speed, 160.0);
        assert_eq!(pos.x, 120.0);
    }

    #[test]
    fn test_finite_comfortable_braking() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, (apply_car_acceleration, car_movement).chain());

        // Northbound car at 150.0 units/s, comfortably braking to 0 (desired_speed = 0.0)
        // comfortable decel = 300.0 units/s^2.
        // For dt = 0.25s, max comfortable decel reduction is 300 * 0.25 = 75.0 units/s.
        let car_id = app
            .world_mut()
            .spawn((
                Car::new(150.0, Vec2::Y, 48.0, 26.0).with_lane(TrafficLane::Northbound),
                GroundPos(Vec2::new(30.0, 0.0)),
                Heading::new(std::f32::consts::FRAC_PI_2),
                CarMotion::new(150.0).with_speeds(150.0, 0.0),
            ))
            .id();

        // 1st step: dt = 0.25s.
        // Current speed reduces by comfortable decel: 150.0 - 300.0 * 0.25 = 75.0 units/s.
        // Car moves by 75.0 * 0.25 = 18.75 units.
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(250));
        app.update();

        let motion = app.world().get::<CarMotion>(car_id).unwrap();
        let pos = app.world().get::<GroundPos>(car_id).unwrap();
        assert_eq!(motion.current_speed, 75.0);
        assert_eq!(pos.y, 18.75);

        // 2nd step: dt = 0.25s.
        // Speed reduces to 0.0 (clamped, does not undershoot below zero).
        // Car moves by 0.0 * 0.25 = 0.0 units.
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_millis(250));
        app.update();

        let motion = app.world().get::<CarMotion>(car_id).unwrap();
        let pos = app.world().get::<GroundPos>(car_id).unwrap();
        assert_eq!(motion.current_speed, 0.0);
        assert_eq!(pos.y, 18.75);
    }

    #[test]
    fn test_finite_emergency_braking() {
        let dt = 0.2;
        let mut motion_comfort = CarMotion::new(240.0).with_speeds(240.0, 0.0);
        let mut motion_emergency = CarMotion::new(240.0).with_speeds(240.0, 0.0);

        motion_comfort.step_with_decel(dt, CAR_COMFORTABLE_DECELERATION);
        motion_emergency.step_with_decel(dt, CAR_EMERGENCY_DECELERATION);

        // Comfort braking reduces by 300 * 0.2 = 60.0 -> 180.0
        assert_eq!(motion_comfort.current_speed, 180.0);
        // Emergency braking reduces by 520 * 0.2 = 104.0 -> 136.0
        assert_eq!(motion_emergency.current_speed, 136.0);
        assert!(motion_emergency.current_speed < motion_comfort.current_speed);

        // Verify a second step to rest under emergency deceleration
        motion_emergency.step_with_decel(dt, CAR_EMERGENCY_DECELERATION);
        assert_eq!(motion_emergency.current_speed, 32.0);
        motion_emergency.step_with_decel(dt, CAR_EMERGENCY_DECELERATION);
        assert_eq!(motion_emergency.current_speed, 0.0);
    }

    #[test]
    fn test_zero_speed_stability() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, (apply_car_acceleration, car_movement).chain());

        let car_id = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::X, 48.0, 26.0).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(50.0, -30.0)),
                Heading::ZERO,
                CarMotion::new(200.0).with_speeds(0.0, 0.0),
            ))
            .id();

        // Advance several seconds through multiple frames
        for _ in 0..10 {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(200));
            app.update();
        }

        let motion = app.world().get::<CarMotion>(car_id).unwrap();
        let pos = app.world().get::<GroundPos>(car_id).unwrap();
        assert_eq!(motion.current_speed, 0.0);
        assert_eq!(pos.x, 50.0);
        assert_eq!(pos.y, -30.0);
    }

    #[test]
    fn test_single_large_frame_bounded_motion() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, (apply_car_acceleration, car_movement).chain());

        // Northbound car at rest (0.0), desired speed is 100.0.
        // A large 10.0-second frame occurs (e.g. lag spike).
        let car_id = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::Y, 48.0, 26.0).with_lane(TrafficLane::Northbound),
                GroundPos(Vec2::new(0.0, 0.0)),
                Heading::new(std::f32::consts::FRAC_PI_2),
                CarMotion::new(240.0).with_speeds(0.0, 100.0),
            ))
            .id();

        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs(10));
        app.update();

        let motion = app.world().get::<CarMotion>(car_id).unwrap();
        let pos = app.world().get::<GroundPos>(car_id).unwrap();

        // Speed must not overshoot desired_speed (100.0), even though accel * dt = 1600.0
        assert_eq!(motion.current_speed, 100.0);

        // Distance moved must not exceed computed speed step * dt = 100.0 * 10.0 = 1000.0
        // (and crucially, must NOT have moved at cruise speed 240.0 * 10 = 2400.0)
        assert_eq!(pos.y, 1000.0);

        // Now test large frame braking from 100.0 to 0.0 with a 5.0s frame
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs(5));
        app.world_mut()
            .get_mut::<CarMotion>(car_id)
            .unwrap()
            .desired_speed = 0.0;
        app.update();

        let motion2 = app.world().get::<CarMotion>(car_id).unwrap();
        let pos2 = app.world().get::<GroundPos>(car_id).unwrap();

        // Speed must clamp at 0.0, never negative
        assert_eq!(motion2.current_speed, 0.0);
        // Vehicle must not move farther or move backward
        assert_eq!(pos2.y, 1000.0);
    }

    #[test]
    fn test_traffic_signal_resource() {
        let signal = TrafficSignal::default();
        assert_eq!(signal.state, TrafficLightState::Green);
        assert_eq!(signal.timer.mode(), TimerMode::Repeating);
        assert_eq!(
            signal.timer.duration(),
            std::time::Duration::from_secs_f32(6.0)
        );

        let yellow_signal = TrafficSignal::new(TrafficLightState::Yellow, 1.5);
        assert_eq!(yellow_signal.state, TrafficLightState::Yellow);
        assert_eq!(
            yellow_signal.timer.duration(),
            std::time::Duration::from_secs_f32(1.5)
        );

        let red_signal = TrafficSignal::new(TrafficLightState::Red, 5.0);
        assert_eq!(red_signal.state, TrafficLightState::Red);
        assert_eq!(
            red_signal.timer.duration(),
            std::time::Duration::from_secs_f32(5.0)
        );
    }

    #[test]
    fn test_traffic_signal_deterministic_transitions() {
        let mut signal = TrafficSignal::default();
        assert_eq!(signal.state, TrafficLightState::Green);

        // Tick 5.9s - still Green
        signal.tick(std::time::Duration::from_secs_f32(5.9));
        assert_eq!(signal.state, TrafficLightState::Green);
        assert!((signal.timer.elapsed_secs() - 5.9).abs() < 1e-5);

        // Tick 0.1s - transitions to Yellow exactly at 6.0s
        signal.tick(std::time::Duration::from_secs_f32(0.1));
        assert_eq!(signal.state, TrafficLightState::Yellow);
        assert_eq!(
            signal.timer.duration(),
            std::time::Duration::from_secs_f32(1.5)
        );
        assert!(signal.timer.elapsed_secs() < 1e-4);

        // Tick 1.5s - transitions to Red
        signal.tick(std::time::Duration::from_secs_f32(1.5));
        assert_eq!(signal.state, TrafficLightState::Red);
        assert_eq!(
            signal.timer.duration(),
            std::time::Duration::from_secs_f32(5.0)
        );
        assert!(signal.timer.elapsed_secs() < 1e-4);

        // Tick 5.0s - transitions back to Green
        signal.tick(std::time::Duration::from_secs_f32(5.0));
        assert_eq!(signal.state, TrafficLightState::Green);
        assert_eq!(
            signal.timer.duration(),
            std::time::Duration::from_secs_f32(6.0)
        );
        assert!(signal.timer.elapsed_secs() < 1e-4);
    }

    #[test]
    fn test_traffic_signal_rollover() {
        let mut signal = TrafficSignal::default();
        // Overshoot green (6.0s) by 0.35s
        signal.tick(std::time::Duration::from_secs_f32(6.35));
        assert_eq!(signal.state, TrafficLightState::Yellow);
        assert!((signal.timer.elapsed_secs() - 0.35).abs() < 1e-4);

        // Overshoot yellow (1.5s) with remaining 1.15s + 0.25s extra
        signal.tick(std::time::Duration::from_secs_f32(1.4));
        assert_eq!(signal.state, TrafficLightState::Red);
        assert!((signal.timer.elapsed_secs() - 0.25).abs() < 1e-4);

        // Multi-phase rollover: from Red (5.0s remaining - 0.25s elapsed = 4.75s remaining),
        // tick 4.75s (finish Red) + 6.0s (finish Green) + 0.4s into Yellow = 11.15s
        signal.tick(std::time::Duration::from_secs_f32(11.15));
        assert_eq!(signal.state, TrafficLightState::Yellow);
        assert!((signal.timer.elapsed_secs() - 0.4).abs() < 1e-4);
    }

    #[test]
    fn test_signal_approach_behavior() {
        let stop_line = 500.0;
        let speed = 240.0;
        let a_comfort = 300.0;
        // d_stop = 240^2 / (2 * 300) = 57600 / 600 = 96.0
        let d_stop = stopping_distance(speed, a_comfort);
        assert_eq!(d_stop, 96.0);

        // 1. Green: always proceed, no signal speed restriction
        let green_res = evaluate_signal_approach(
            TrafficLightState::Green,
            speed,
            300.0,
            stop_line,
            false,
            a_comfort,
        );
        assert_eq!(green_res.decision, SignalApproachDecision::Proceed);
        assert!(!green_res.committed_to_cross);

        // 2. Yellow:
        // Can stop comfortably before line (e.g. dist_to_line = 100.0 > 96.0)
        let yellow_stop = evaluate_signal_approach(
            TrafficLightState::Yellow,
            speed,
            stop_line - 100.0,
            stop_line,
            false,
            a_comfort,
        );
        assert_eq!(yellow_stop.decision, SignalApproachDecision::Stop);
        assert!(!yellow_stop.committed_to_cross);

        // Boundary test: dist_to_line = d_stop + 0.01 (96.01) -> can stop comfortably
        let yellow_just_outside = evaluate_signal_approach(
            TrafficLightState::Yellow,
            speed,
            stop_line - (d_stop + 0.01),
            stop_line,
            false,
            a_comfort,
        );
        assert_eq!(yellow_just_outside.decision, SignalApproachDecision::Stop);
        assert!(!yellow_just_outside.committed_to_cross);

        // Exact boundary test: dist_to_line == d_stop (96.0) -> can stop comfortably
        let yellow_boundary = evaluate_signal_approach(
            TrafficLightState::Yellow,
            speed,
            stop_line - d_stop,
            stop_line,
            false,
            a_comfort,
        );
        assert_eq!(yellow_boundary.decision, SignalApproachDecision::Stop);
        assert!(!yellow_boundary.committed_to_cross);

        // Boundary test: dist_to_line = d_stop - 0.01 (95.99) -> cannot stop comfortably -> commits
        let yellow_just_inside = evaluate_signal_approach(
            TrafficLightState::Yellow,
            speed,
            stop_line - (d_stop - 0.01),
            stop_line,
            false,
            a_comfort,
        );
        assert_eq!(yellow_just_inside.decision, SignalApproachDecision::Proceed);
        assert!(yellow_just_inside.committed_to_cross);

        // Cannot stop comfortably before line (e.g. dist_to_line = 90.0 < 96.0) -> commits
        let yellow_go = evaluate_signal_approach(
            TrafficLightState::Yellow,
            speed,
            stop_line - 90.0,
            stop_line,
            false,
            a_comfort,
        );
        assert_eq!(yellow_go.decision, SignalApproachDecision::Proceed);
        assert!(yellow_go.committed_to_cross);
        // 3. Red:
        // Normal car before line must stop
        let red_stop = evaluate_signal_approach(
            TrafficLightState::Red,
            speed,
            stop_line - 50.0,
            stop_line,
            false,
            a_comfort,
        );
        assert_eq!(red_stop.decision, SignalApproachDecision::Stop);
        assert!(!red_stop.committed_to_cross);

        // Car that committed during yellow proceeds through red
        let red_committed = evaluate_signal_approach(
            TrafficLightState::Red,
            speed,
            stop_line - 50.0,
            stop_line,
            true,
            a_comfort,
        );
        assert_eq!(red_committed.decision, SignalApproachDecision::Proceed);
        assert!(red_committed.committed_to_cross);

        // Car whose front bumper already crossed stop line proceeds
        let red_crossed = evaluate_signal_approach(
            TrafficLightState::Red,
            speed,
            stop_line + 5.0,
            stop_line,
            false,
            a_comfort,
        );
        assert_eq!(red_crossed.decision, SignalApproachDecision::Proceed);
    }

    #[test]
    fn test_yellow_commitment_lifecycle_and_clearing_after_rear_bumper() {
        let stop_line = 500.0;
        let car_len = CAR_LENGTH; // 48.0
        let half_len = car_len / 2.0; // 24.0

        let mut car =
            Car::new(CAR_SPEED, Vec2::X, car_len, CAR_WIDTH).with_lane(TrafficLane::Eastbound);
        assert!(!car.committed_to_cross);

        // 1. Approaching yellow: cannot stop comfortably -> commits
        let center_p1 = stop_line - half_len - 30.0; // front bumper at stop_line - 30.0 (< 96.0 d_stop)
        let eval1 = evaluate_signal_approach(
            TrafficLightState::Yellow,
            car.speed,
            car.front_progress(center_p1),
            stop_line,
            car.committed_to_cross,
            CAR_COMFORTABLE_DECELERATION,
        );
        assert_eq!(eval1.decision, SignalApproachDecision::Proceed);
        assert!(eval1.committed_to_cross);
        car.committed_to_cross = eval1.committed_to_cross;
        assert!(car.committed_to_cross);

        // 2. Light turns Red while car is crossing:
        // Front bumper is past stop line, but rear bumper is still behind
        let center_p2 = stop_line; // front = stop_line + 24.0, rear = stop_line - 24.0
        assert!(car.front_progress(center_p2) > stop_line);
        assert!(car.rear_progress(center_p2) < stop_line);

        // Updating commitment does NOT clear yet because rear bumper is behind
        car.update_crossing_commitment(center_p2, stop_line);
        assert!(
            car.committed_to_cross,
            "Commitment must be retained while inside intersection"
        );

        // Red evaluation still permits crossing so car is not stranded
        let eval2 = evaluate_signal_approach(
            TrafficLightState::Red,
            car.speed,
            car.front_progress(center_p2),
            stop_line,
            car.committed_to_cross,
            CAR_COMFORTABLE_DECELERATION,
        );
        assert_eq!(eval2.decision, SignalApproachDecision::Proceed);

        // 3. Rear bumper passes the stop line (e.g. center = stop_line + 25.0 -> rear = stop_line + 1.0)
        let center_p3 = stop_line + half_len + 1.0;
        assert!(car.rear_progress(center_p3) > stop_line);
        car.update_crossing_commitment(center_p3, stop_line);
        assert!(
            !car.committed_to_cross,
            "Commitment must be cleared once rear bumper clears line"
        );
    }

    #[test]
    fn test_red_after_yellow_commitment_and_non_commitment_cases() {
        let stop_line = 600.0;
        let speed = CAR_SPEED; // 240.0
        let a_comfort = CAR_COMFORTABLE_DECELERATION; // 300.0
        let d_stop = stopping_distance(speed, a_comfort); // 96.0
        let car_len = CAR_LENGTH; // 48.0
        let half_len = car_len / 2.0; // 24.0

        // --- Case 1: Car A (can stop at yellow, does NOT commit) ---
        let mut car_a = Car::new(speed, Vec2::X, car_len, CAR_WIDTH);
        let center_a = stop_line - half_len - (d_stop + 10.0); // front bumper is d_stop + 10.0 before line
        let yellow_a = evaluate_signal_approach(
            TrafficLightState::Yellow,
            car_a.speed,
            car_a.front_progress(center_a),
            stop_line,
            car_a.committed_to_cross,
            a_comfort,
        );
        assert_eq!(yellow_a.decision, SignalApproachDecision::Stop);
        assert!(!yellow_a.committed_to_cross);
        car_a.committed_to_cross = yellow_a.committed_to_cross;

        // Signal transitions to Red: Car A is still approaching before line
        let red_a = evaluate_signal_approach(
            TrafficLightState::Red,
            car_a.speed,
            car_a.front_progress(center_a),
            stop_line,
            car_a.committed_to_cross,
            a_comfort,
        );
        assert_eq!(
            red_a.decision,
            SignalApproachDecision::Stop,
            "Car A without commitment must stop on red"
        );
        assert!(!red_a.committed_to_cross);

        // --- Case 2: Car B (cannot stop at yellow, commits to crossing) ---
        let mut car_b = Car::new(speed, Vec2::X, car_len, CAR_WIDTH);
        let center_b1 = stop_line - half_len - (d_stop - 20.0); // front bumper is d_stop - 20.0 before line
        let yellow_b = evaluate_signal_approach(
            TrafficLightState::Yellow,
            car_b.speed,
            car_b.front_progress(center_b1),
            stop_line,
            car_b.committed_to_cross,
            a_comfort,
        );
        assert_eq!(yellow_b.decision, SignalApproachDecision::Proceed);
        assert!(yellow_b.committed_to_cross);
        car_b.committed_to_cross = yellow_b.committed_to_cross;

        // Signal transitions to Red BEFORE Car B's front bumper crosses stop line:
        // Car B is committed, so Red still permits proceeding
        let red_b1 = evaluate_signal_approach(
            TrafficLightState::Red,
            car_b.speed,
            car_b.front_progress(center_b1),
            stop_line,
            car_b.committed_to_cross,
            a_comfort,
        );
        assert_eq!(
            red_b1.decision,
            SignalApproachDecision::Proceed,
            "Committed Car B must proceed on red"
        );
        assert!(red_b1.committed_to_cross);

        // Car B moves partially into intersection: front bumper crossed, rear bumper still behind
        let center_b2 = stop_line; // front = stop_line + 24.0, rear = stop_line - 24.0
        car_b.update_crossing_commitment(center_b2, stop_line);
        assert!(
            car_b.committed_to_cross,
            "Commitment must stay active while rear bumper is behind line"
        );

        let red_b2 = evaluate_signal_approach(
            TrafficLightState::Red,
            car_b.speed,
            car_b.front_progress(center_b2),
            stop_line,
            car_b.committed_to_cross,
            a_comfort,
        );
        assert_eq!(
            red_b2.decision,
            SignalApproachDecision::Proceed,
            "Car B must continue across intersection on red"
        );

        // Car B rear bumper fully clears the stop line
        let center_b3 = stop_line + half_len + 10.0;
        car_b.update_crossing_commitment(center_b3, stop_line);
        assert!(
            !car_b.committed_to_cross,
            "Commitment must be cleared once rear bumper passes line"
        );
    }

    #[test]
    fn test_update_car_signal_commitments_system() {
        let mut app = App::new();
        let geom = RoadGeometry::default();
        app.insert_resource(geom.clone());
        app.insert_resource(TrafficSignal::new(
            TrafficLightState::Yellow,
            SIGNAL_YELLOW_SECS,
        ));
        app.add_systems(Update, update_car_signal_commitments);

        let stop_pos = geom.stop_line_position(TrafficLane::Eastbound);
        let stop_line = lane_progress(TrafficLane::Eastbound, stop_pos, &geom);
        let _d_stop = stopping_distance(CAR_SPEED, CAR_COMFORTABLE_DECELERATION);

        // Position car so front bumper is 50.0 units before stop line (less than d_stop = 96.0)
        let front_p = stop_line - 50.0;
        let center_p = front_p - CAR_LENGTH / 2.0;
        let x_spawn = -geom.world_width / 2.0 - SPAWN_MARGIN;
        let car_pos = Vec2::new(x_spawn + center_p, geom.east_lane_y());

        let car_entity = app
            .world_mut()
            .spawn((
                Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH)
                    .with_lane(TrafficLane::Eastbound),
                GroundPos(car_pos),
                Heading::ZERO,
                CarMotion::new(CAR_SPEED),
            ))
            .id();

        // 1. In Yellow, system evaluates and commits car to crossing
        app.update();
        let car = app.world().get::<Car>(car_entity).unwrap();
        assert!(
            car.committed_to_cross,
            "Car must commit to cross during yellow when inside stopping distance"
        );

        // 2. Signal transitions to Red; car moves into intersection but rear bumper still behind stop line
        {
            let mut sig = app.world_mut().resource_mut::<TrafficSignal>();
            sig.state = TrafficLightState::Red;
        }
        // Move car center exactly to stop line -> rear bumper is at stop_line - 24.0
        {
            let mut gpos = app.world_mut().get_mut::<GroundPos>(car_entity).unwrap();
            gpos.0.x = x_spawn + stop_line;
        }
        app.update();

        let car = app.world().get::<Car>(car_entity).unwrap();
        assert!(
            car.committed_to_cross,
            "Commitment must remain while rear bumper is behind stop line during red"
        );

        // 3. Car advances so rear bumper passes stop line
        {
            let mut gpos = app.world_mut().get_mut::<GroundPos>(car_entity).unwrap();
            gpos.0.x = x_spawn + stop_line + CAR_LENGTH / 2.0 + 5.0;
        }
        app.update();

        let car = app.world().get::<Car>(car_entity).unwrap();
        assert!(
            !car.committed_to_cross,
            "Commitment must be cleared once rear bumper clears stop line"
        );
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

        let west_pos = TrafficLane::Westbound.spawn_position(&geom);
        assert_eq!(west_pos.x, geom.curve_west_lane_x());
        assert_eq!(west_pos.y, HALF_WORLD_HEIGHT + SPAWN_MARGIN);

        let south_pos = TrafficLane::Southbound.spawn_position(&geom);
        assert_eq!(south_pos.x, geom.south_lane_x());
        assert_eq!(south_pos.y, HALF_WORLD_HEIGHT + SPAWN_MARGIN);

        let north_pos = TrafficLane::Northbound.spawn_position(&geom);
        assert_eq!(north_pos.x, geom.north_lane_x());
        assert_eq!(north_pos.y, -HALF_WORLD_HEIGHT - SPAWN_MARGIN);
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
                CarMotion::new(200.0),
            ))
            .id();

        let car_s = app
            .world_mut()
            .spawn((
                Car::new(100.0, Vec2::new(0.0, -1.0), 26.0, 48.0),
                GroundPos(Vec2::new(-30.0, 100.0)),
                Heading::ZERO,
                CarMotion::new(100.0),
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
                CarMotion::new(200.0),
            ))
            .id();

        // Exited car (+X eastbound past despawn)
        let despawn_x = HALF_WORLD_WIDTH + DESPAWN_MARGIN;
        let car_dead_e = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::new(1.0, 0.0), 48.0, 26.0),
                GroundPos(Vec2::new(despawn_x + 1.0, -30.0)),
                CarMotion::new(200.0),
            ))
            .id();

        // Exited car (-Y southbound past despawn)
        let despawn_y = HALF_WORLD_HEIGHT + DESPAWN_MARGIN;
        let car_dead_s = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::new(0.0, -1.0), 48.0, 26.0),
                GroundPos(Vec2::new(-30.0, -despawn_y - 5.0)),
                CarMotion::new(200.0),
            ))
            .id();
        app.add_systems(Update, car_cleanup);
        app.update();

        assert!(app.world().get_entity(car_alive).is_ok());
        assert!(app.world().get_entity(car_dead_e).is_err());
        assert!(app.world().get_entity(car_dead_s).is_err());
    }

    #[test]
    fn test_crashed_car_cleanup_after_duration() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, (tick_crash_cleanup, car_cleanup).chain());

        let crashed_car = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::X, 48.0, 26.0),
                GroundPos(Vec2::new(0.0, 0.0)),
                CarMotion::new(0.0),
                Crashed::default(),
            ))
            .id();

        // Advance 2.0s (< 3.0s CRASH_CLEANUP_SECS)
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.update();
        assert!(
            app.world().get_entity(crashed_car).is_ok(),
            "Crashed car should remain before duration expires"
        );

        // Advance another 1.1s (total 3.1s > 3.0s)
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(1100));
        app.update();
        assert!(
            app.world().get_entity(crashed_car).is_err(),
            "Crashed car should be despawned after duration expires"
        );
    }

    #[test]
    fn test_car_spawning_adds_mesh_and_material() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<RoadGeometry>();

        let geom = RoadGeometry::default();

        type SpawnCarState<'w, 's> = bevy::ecs::system::SystemState<Commands<'w, 's>>;
        let car_id = {
            let mut system_state = SpawnCarState::new(app.world_mut());
            let mut cmd = system_state.get_mut(app.world_mut()).unwrap();
            let id = spawn_car_in_lane(&mut cmd, &geom, TrafficLane::Eastbound, 0);
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
        let motion_ref = app
            .world()
            .get::<CarMotion>(car_id)
            .expect("CarMotion component should exist");
        assert_eq!(motion_ref.current_speed, CAR_SPEED);
        assert_eq!(motion_ref.desired_speed, CAR_SPEED);
    }

    #[test]
    fn test_spawn_car_in_each_lane_heading_and_dimensions() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<RoadGeometry>();

        let geom = RoadGeometry::default();

        type SpawnCarState<'w, 's> = bevy::ecs::system::SystemState<Commands<'w, 's>>;

        for lane in TrafficLane::ALL {
            let car_id = {
                let mut system_state = SpawnCarState::new(app.world_mut());
                let mut cmd = system_state.get_mut(app.world_mut()).unwrap();
                let id = spawn_car_in_lane(&mut cmd, &geom, lane, 0);
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
            let motion = app.world().get::<CarMotion>(car_id).unwrap();
            assert_eq!(motion.current_speed, CAR_SPEED);
            assert_eq!(motion.desired_speed, CAR_SPEED);
        }
    }

    #[test]
    fn test_traffic_plugin_periodic_spawn_in_app() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin)
            .init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
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
    fn test_spawn_cars_deferral_when_footprint_occupied() {
        let mut app = App::new();
        let geom = RoadGeometry::default();
        let east_spawn = TrafficLane::Eastbound.spawn_position(&geom);

        app.add_plugins(StatesPlugin)
            .init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .insert_resource(geom)
            .init_state::<AppState>()
            .add_plugins(TrafficPlugin);

        // Spawn an existing blocking car in the Eastbound lane occupying the spawn footprint
        // (e.g. stopped right at east_spawn)
        let blocker = app
            .world_mut()
            .spawn((
                Car::new(0.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(0.0),
                GroundPos(east_spawn),
                Heading(0.0),
            ))
            .id();

        app.update();
        assert_eq!(
            app.world_mut().query::<&Car>().iter(app.world()).count(),
            1,
            "Only blocker initially exists"
        );

        // Advance by spawn interval (1.25s)
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(1250));
        app.update();

        // Spawn should be deferred: still only 1 car, deferred flag true, lane still Eastbound
        assert_eq!(
            app.world_mut().query::<&Car>().iter(app.world()).count(),
            1,
            "Spawn must be deferred because footprint is occupied"
        );
        let spawn_timer = app.world().resource::<SpawnTimer>();
        assert!(spawn_timer.deferred, "SpawnTimer should be marked deferred");
        assert_eq!(spawn_timer.current_lane(), TrafficLane::Eastbound);

        // Move blocker far enough down the lane so bumper gap >= CAR_MIN_FOLLOW_GAP
        // Spawn front is at progress 24.0. Blocker rear needs to be >= 24.0 + 14.0 = 38.0.
        // Blocker center needs to be >= 38.0 + 24.0 = 62.0.
        // Move blocker to east_spawn.x + 100.0:
        app.world_mut().get_mut::<GroundPos>(blocker).unwrap().0 =
            east_spawn + Vec2::new(100.0, 0.0);

        // Advance a small frame (e.g. 50ms)
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();

        // Now deferred car should have spawned!
        assert_eq!(
            app.world_mut().query::<&Car>().iter(app.world()).count(),
            2,
            "Deferred car should spawn once bumper clearance exists"
        );
        let spawn_timer_after = app.world().resource::<SpawnTimer>();
        assert!(
            !spawn_timer_after.deferred,
            "Deferred flag should clear after spawning"
        );
        assert_eq!(
            spawn_timer_after.current_lane(),
            TrafficLane::Westbound,
            "Lane index should advance to Westbound after spawning"
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
                CarMotion::new(200.0),
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
                CarMotion::new(200.0),
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
                CarMotion::new(240.0),
            ))
            .id();

        // Westbound car starts at x = 630, y = 560
        let car_w = app
            .world_mut()
            .spawn((
                Car::new(240.0, -Vec2::Y, 48.0, 26.0).with_lane(TrafficLane::Westbound),
                GroundPos(Vec2::new(geom.curve_west_lane_x(), 560.0)),
                Heading::new(-std::f32::consts::FRAC_PI_2),
                CarMotion::new(240.0),
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

    #[test]
    fn test_lane_progress_all_lanes_monotonic_and_continuous() {
        let geom = RoadGeometry::default();

        // 1. Eastbound progress verification
        let eb = TrafficLane::Eastbound;
        let spawn_eb = eb.spawn_position(&geom);
        assert_eq!(lane_progress(eb, spawn_eb, &geom), 0.0);

        let x_curve = geom.curve_start_x; // 160.0
        let r_east = geom.curve_east_lane_radius(); // 530.0
        let y_center = geom.curve_center.y; // 500.0
        let y_east = geom.east_lane_y(); // -30.0

        // West straight segment
        let p_straight_1 = lane_progress(eb, Vec2::new(-400.0, y_east), &geom);
        let p_straight_2 = lane_progress(eb, Vec2::new(0.0, y_east), &geom);
        let p_straight_end = lane_progress(eb, Vec2::new(x_curve, y_east), &geom);
        assert!(0.0 < p_straight_1);
        assert!(p_straight_1 < p_straight_2);
        assert!(p_straight_2 < p_straight_end);

        // Curve entry continuity
        let p_curve_start = lane_progress(
            eb,
            Vec2::new(
                x_curve + r_east * 0.0_f32.sin(),
                y_center - r_east * 0.0_f32.cos(),
            ),
            &geom,
        );
        assert_eq!(p_straight_end, p_curve_start);

        // Curve intermediate points
        let theta1 = std::f32::consts::FRAC_PI_4;
        let p_curve_mid = lane_progress(
            eb,
            Vec2::new(
                x_curve + r_east * theta1.sin(),
                y_center - r_east * theta1.cos(),
            ),
            &geom,
        );
        assert!(p_curve_start < p_curve_mid);

        // Curve exit continuity
        let p_curve_end = lane_progress(
            eb,
            Vec2::new(
                x_curve + r_east * std::f32::consts::FRAC_PI_2.sin(),
                y_center - r_east * std::f32::consts::FRAC_PI_2.cos(),
            ),
            &geom,
        );
        let p_north_start = lane_progress(eb, Vec2::new(x_curve + r_east, y_center), &geom);
        assert!((p_curve_end - p_north_start).abs() < 1e-4);
        assert!(p_curve_mid < p_curve_end);

        // North straight segment
        let p_north_1 = lane_progress(eb, Vec2::new(x_curve + r_east, y_center + 50.0), &geom);
        let p_north_2 = lane_progress(eb, Vec2::new(x_curve + r_east, y_center + 100.0), &geom);
        assert!(p_north_start < p_north_1);
        assert!(p_north_1 < p_north_2);

        // 2. Westbound progress verification
        let wb = TrafficLane::Westbound;
        let spawn_wb = wb.spawn_position(&geom);
        assert_eq!(lane_progress(wb, spawn_wb, &geom), 0.0);

        let r_west = geom.curve_west_lane_radius(); // 470.0
        let x_west_lane = x_curve + r_west; // 630.0
        let y_west = geom.west_lane_y(); // 30.0

        // North straight segment (moving south)
        let p_wb_north_1 = lane_progress(wb, Vec2::new(x_west_lane, 540.0), &geom);
        let p_wb_north_2 = lane_progress(wb, Vec2::new(x_west_lane, 520.0), &geom);
        let p_wb_north_end = lane_progress(wb, Vec2::new(x_west_lane, y_center), &geom);
        assert!(0.0 < p_wb_north_1);
        assert!(p_wb_north_1 < p_wb_north_2);
        assert!(p_wb_north_2 < p_wb_north_end);

        // Curve entry continuity (from north into curve at y = curve_center.y)
        let theta_wb_start = std::f32::consts::FRAC_PI_2;
        let p_wb_curve_start = lane_progress(
            wb,
            Vec2::new(
                x_curve + r_west * theta_wb_start.sin(),
                y_center - r_west * theta_wb_start.cos(),
            ),
            &geom,
        );
        assert!((p_wb_north_end - p_wb_curve_start).abs() < 1e-4);

        // Curve intermediate points (moving towards theta = 0)
        let theta_wb_mid = std::f32::consts::FRAC_PI_4;
        let p_wb_curve_mid = lane_progress(
            wb,
            Vec2::new(
                x_curve + r_west * theta_wb_mid.sin(),
                y_center - r_west * theta_wb_mid.cos(),
            ),
            &geom,
        );
        assert!(p_wb_curve_start < p_wb_curve_mid);

        // Curve exit continuity (into west straight at x = curve_start_x, y = west_lane_y)
        let theta_wb_end = 0.0_f32;
        let p_wb_curve_end = lane_progress(
            wb,
            Vec2::new(
                x_curve + r_west * theta_wb_end.sin(),
                y_center - r_west * theta_wb_end.cos(),
            ),
            &geom,
        );
        let p_wb_west_start = lane_progress(wb, Vec2::new(x_curve, y_west), &geom);
        assert!((p_wb_curve_end - p_wb_west_start).abs() < 1e-4);
        assert!(p_wb_curve_mid < p_wb_curve_end);

        // West straight segment (moving west, x decreasing)
        let p_wb_west_1 = lane_progress(wb, Vec2::new(100.0, y_west), &geom);
        let p_wb_west_2 = lane_progress(wb, Vec2::new(0.0, y_west), &geom);
        let p_wb_west_3 = lane_progress(wb, Vec2::new(-200.0, y_west), &geom);
        assert!(p_wb_west_start < p_wb_west_1);
        assert!(p_wb_west_1 < p_wb_west_2);
        assert!(p_wb_west_2 < p_wb_west_3);

        // 3. Southbound direct vertical progress
        let sb = TrafficLane::Southbound;
        let spawn_sb = sb.spawn_position(&geom);
        assert_eq!(lane_progress(sb, spawn_sb, &geom), 0.0);
        let p_sb_1 = lane_progress(sb, Vec2::new(geom.south_lane_x(), 200.0), &geom);
        let p_sb_2 = lane_progress(sb, Vec2::new(geom.south_lane_x(), 0.0), &geom);
        let p_sb_3 = lane_progress(sb, Vec2::new(geom.south_lane_x(), -200.0), &geom);
        assert!(0.0 < p_sb_1);
        assert!(p_sb_1 < p_sb_2);
        assert!(p_sb_2 < p_sb_3);

        // 4. Northbound direct vertical progress
        let nb = TrafficLane::Northbound;
        let spawn_nb = nb.spawn_position(&geom);
        assert_eq!(lane_progress(nb, spawn_nb, &geom), 0.0);
        let p_nb_1 = lane_progress(nb, Vec2::new(geom.north_lane_x(), -200.0), &geom);
        let p_nb_2 = lane_progress(nb, Vec2::new(geom.north_lane_x(), 0.0), &geom);
        let p_nb_3 = lane_progress(nb, Vec2::new(geom.north_lane_x(), 200.0), &geom);
        assert!(0.0 < p_nb_1);
        assert!(p_nb_1 < p_nb_2);
        assert!(p_nb_2 < p_nb_3);
    }

    #[test]
    fn test_stop_line_progress_all_lanes() {
        let geom = RoadGeometry::default();
        let half_car = CAR_LENGTH / 2.0;

        for lane in TrafficLane::ALL {
            let stop_prog = stop_line_progress(lane, &geom);
            let bound_prog = lane_progress(lane, geom.stop_line_position(lane), &geom);

            // Progress of stopped car center must be exactly one half-car plus buffer before the intersection boundary
            assert_eq!(stop_prog, bound_prog - (half_car + STOP_LINE_BUFFER));
            assert_eq!(lane.stop_line_progress(&geom), stop_prog);
            assert_eq!(geom.stop_line_progress(lane), stop_prog);

            // Front bumper position is at stop_prog + half_car
            let front_bumper_prog = stop_prog + half_car;
            assert_eq!(bound_prog - front_bumper_prog, STOP_LINE_BUFFER);
            assert!(front_bumper_prog < bound_prog);
        }
    }

    #[test]
    fn test_front_rear_progress_and_bumper_gaps() {
        let car = Car::new(240.0, Vec2::X, 48.0, 26.0);
        let center_p = 500.0;

        // Check front and rear helpers
        let fp = front_progress(center_p, car.length);
        let rp = rear_progress(center_p, car.length);
        assert_eq!(fp, 524.0);
        assert_eq!(rp, 476.0);
        assert_eq!(car.front_progress(center_p), 524.0);
        assert_eq!(car.rear_progress(center_p), 476.0);
        assert_eq!(fp - rp, car.length);

        // Lead and follower bumper gap test
        let lead_car = Car::new(200.0, Vec2::X, 50.0, 26.0);
        let follower_car = Car::new(240.0, Vec2::X, 40.0, 26.0);

        let lead_center_progress = 600.0;
        let follower_center_progress = 520.0;

        let center_distance = lead_center_progress - follower_center_progress; // 80.0
        let lead_rear = lead_car.rear_progress(lead_center_progress); // 600 - 25 = 575.0
        let follower_front = follower_car.front_progress(follower_center_progress); // 520 + 20 = 540.0
        let bumper_gap = lead_rear - follower_front; // 575 - 540 = 35.0

        // Bumper gap accounts for half-lengths of both vehicles
        assert_eq!(
            bumper_gap,
            center_distance - (lead_car.length / 2.0 + follower_car.length / 2.0)
        );
        assert_eq!(bumper_gap, 35.0);

        // Collision / contact condition: bumper_gap <= 0.0
        let touching_follower_progress = lead_rear - follower_car.length / 2.0; // 575 - 20 = 555.0
        assert_eq!(
            lead_car.rear_progress(lead_center_progress)
                - follower_car.front_progress(touching_follower_progress),
            0.0
        );
    }

    #[test]
    fn test_monotonic_progress_across_boundaries_and_stop_lines() {
        let geom = RoadGeometry::default();
        let eps = 1.0;

        // 1. Eastbound straight/curve entry boundary (x = 160.0, y = -30.0)
        let eb = TrafficLane::Eastbound;
        let r_east = geom.curve_east_lane_radius();
        let p_eb_entry_before = lane_progress(
            eb,
            Vec2::new(geom.curve_start_x - eps, geom.east_lane_y()),
            &geom,
        );
        let p_eb_entry_boundary =
            lane_progress(eb, Vec2::new(geom.curve_start_x, geom.east_lane_y()), &geom);
        let dtheta_east = eps / r_east;
        let p_eb_entry_after = lane_progress(
            eb,
            Vec2::new(
                geom.curve_start_x + r_east * dtheta_east.sin(),
                geom.curve_center.y - r_east * dtheta_east.cos(),
            ),
            &geom,
        );
        assert!(p_eb_entry_before < p_eb_entry_boundary);
        assert!(p_eb_entry_boundary < p_eb_entry_after);
        assert!((p_eb_entry_boundary - p_eb_entry_before - eps).abs() < 1e-3);
        assert!((p_eb_entry_after - p_eb_entry_boundary - eps).abs() < 1e-3);

        // 2. Eastbound curve/straight exit boundary (x = 690.0, y = 500.0)
        let theta_exit = std::f32::consts::FRAC_PI_2 - dtheta_east;
        let p_eb_exit_before = lane_progress(
            eb,
            Vec2::new(
                geom.curve_start_x + r_east * theta_exit.sin(),
                geom.curve_center.y - r_east * theta_exit.cos(),
            ),
            &geom,
        );
        let p_eb_exit_boundary = lane_progress(
            eb,
            Vec2::new(geom.curve_start_x + r_east, geom.curve_center.y),
            &geom,
        );
        let p_eb_exit_after = lane_progress(
            eb,
            Vec2::new(geom.curve_start_x + r_east, geom.curve_center.y + eps),
            &geom,
        );
        assert!(p_eb_exit_before < p_eb_exit_boundary);
        assert!(p_eb_exit_boundary < p_eb_exit_after);
        assert!((p_eb_exit_boundary - p_eb_exit_before - eps).abs() < 1e-3);
        assert!((p_eb_exit_after - p_eb_exit_boundary - eps).abs() < 1e-3);

        // 3. Westbound straight/curve entry boundary (x = 630.0, y = 500.0)
        let wb = TrafficLane::Westbound;
        let r_west = geom.curve_west_lane_radius();
        let p_wb_entry_before = lane_progress(
            wb,
            Vec2::new(geom.curve_start_x + r_west, geom.curve_center.y + eps),
            &geom,
        );
        let p_wb_entry_boundary = lane_progress(
            wb,
            Vec2::new(geom.curve_start_x + r_west, geom.curve_center.y),
            &geom,
        );
        let dtheta_west = eps / r_west;
        let theta_wb_entry = std::f32::consts::FRAC_PI_2 - dtheta_west;
        let p_wb_entry_after = lane_progress(
            wb,
            Vec2::new(
                geom.curve_start_x + r_west * theta_wb_entry.sin(),
                geom.curve_center.y - r_west * theta_wb_entry.cos(),
            ),
            &geom,
        );
        assert!(p_wb_entry_before < p_wb_entry_boundary);
        assert!(p_wb_entry_boundary < p_wb_entry_after);
        assert!((p_wb_entry_boundary - p_wb_entry_before - eps).abs() < 1e-3);
        assert!((p_wb_entry_after - p_wb_entry_boundary - eps).abs() < 1e-3);

        // 4. Westbound curve/straight exit boundary (x = 160.0, y = 30.0)
        let p_wb_exit_before = lane_progress(
            wb,
            Vec2::new(
                geom.curve_start_x + r_west * dtheta_west.sin(),
                geom.curve_center.y - r_west * dtheta_west.cos(),
            ),
            &geom,
        );
        let p_wb_exit_boundary =
            lane_progress(wb, Vec2::new(geom.curve_start_x, geom.west_lane_y()), &geom);
        let p_wb_exit_after = lane_progress(
            wb,
            Vec2::new(geom.curve_start_x - eps, geom.west_lane_y()),
            &geom,
        );
        assert!(p_wb_exit_before < p_wb_exit_boundary);
        assert!(p_wb_exit_boundary < p_wb_exit_after);
        assert!((p_wb_exit_boundary - p_wb_exit_before - eps).abs() < 1e-3);
        assert!((p_wb_exit_after - p_wb_exit_boundary - eps).abs() < 1e-3);

        // 5. All four stop lines: monotonic ordering before, at, and after stop line
        for lane in TrafficLane::ALL {
            let stop_p = stop_line_progress(lane, &geom);
            let bound_pos = geom.stop_line_position(lane);
            let bound_p = lane_progress(lane, bound_pos, &geom);

            let p_before_stop = stop_p - eps;
            let p_after_stop = stop_p + eps;

            assert!(p_before_stop < stop_p);
            assert!(stop_p < p_after_stop);
            assert!(p_after_stop < bound_p);

            // Front bumper at stop line is buffer distance before intersection boundary
            let front_at_stop = front_progress(stop_p, CAR_LENGTH);
            assert_eq!(bound_p - front_at_stop, STOP_LINE_BUFFER);

            // Advance by buffer distance touches intersection boundary exactly
            let front_at_bound = front_progress(stop_p + STOP_LINE_BUFFER, CAR_LENGTH);
            assert_eq!(front_at_bound, bound_p);
        }
    }

    #[test]
    fn test_lane_grouping_progress_ordering_and_bumper_gaps() {
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();

        // Check bumper_gap formula directly
        assert_eq!(bumper_gap(100.0, 70.0), 30.0);
        assert_eq!(bumper_gap(100.0, 100.0), 0.0);
        assert_eq!(bumper_gap(100.0, 105.0), -5.0); // Overlapping

        let mut app = App::new();
        app.init_resource::<Time>();
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.1));
        }
        app.insert_resource(geom.clone());

        // Spawn Eastbound cars in non-sorted order
        let car_rear = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(-40.0, east_y)),
                Heading::ZERO,
                CarMotion::new(240.0),
            ))
            .id();

        let car_lead = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(100.0, east_y)),
                Heading::ZERO,
                CarMotion::new(240.0),
            ))
            .id();

        let car_mid = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(40.0, east_y)),
                Heading::ZERO,
                CarMotion::new(240.0),
            ))
            .id();

        // Spawn Southbound car in a different lane
        let car_south = app
            .world_mut()
            .spawn((
                Car::new(240.0, -Vec2::Y, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Southbound),
                GroundPos(Vec2::new(geom.south_lane_x(), 200.0)),
                Heading::new(-std::f32::consts::FRAC_PI_2),
                CarMotion::new(240.0),
            ))
            .id();

        app.add_systems(Update, plan_car_motion);
        app.update();

        // Verify entries collected and ordered
        let p_lead = lane_progress(TrafficLane::Eastbound, Vec2::new(100.0, east_y), &geom);
        let p_mid = lane_progress(TrafficLane::Eastbound, Vec2::new(40.0, east_y), &geom);
        let p_rear = lane_progress(TrafficLane::Eastbound, Vec2::new(-40.0, east_y), &geom);
        assert!(p_lead > p_mid);
        assert!(p_mid > p_rear);

        // Assert directly on what plan_car_motion computed and wrote to CarMotion
        let motion_lead = app.world().get::<CarMotion>(car_lead).unwrap();
        let motion_mid = app.world().get::<CarMotion>(car_mid).unwrap();
        let motion_rear = app.world().get::<CarMotion>(car_rear).unwrap();
        let motion_south = app.world().get::<CarMotion>(car_south).unwrap();

        // car_lead is furthest along Eastbound lane -> leader -> None
        assert_eq!(motion_lead.leader_gap, None);

        // car_mid is behind car_lead
        // Expected gap: (100.0 - 24.0) - (40.0 + 24.0) = 76.0 - 64.0 = 12.0
        let expected_gap_mid = (100.0 - CAR_LENGTH / 2.0) - (40.0 + CAR_LENGTH / 2.0);
        assert_eq!(motion_mid.leader_gap, Some(expected_gap_mid));
        assert_eq!(motion_mid.leader_gap, Some(12.0));

        // car_rear is behind car_mid
        // Expected gap: (40.0 - 24.0) - (-40.0 + 24.0) = 16.0 - (-16.0) = 32.0
        let expected_gap_rear = (40.0 - CAR_LENGTH / 2.0) - (-40.0 + CAR_LENGTH / 2.0);
        assert_eq!(motion_rear.leader_gap, Some(expected_gap_rear));
        assert_eq!(motion_rear.leader_gap, Some(32.0));

        // car_south is in a separate lane, so it must NOT group with Eastbound cars
        // It is the sole car in Southbound lane -> leader -> None
        assert_eq!(motion_south.leader_gap, None);
    }

    #[test]
    fn test_scratch_vectors_reused_through_bevy_local() {
        // 1. Verify LaneCarScratch behavior: clear() preserves allocated capacity
        let mut scratch = LaneCarScratch::default();
        for lane in &scratch.lanes {
            assert_eq!(lane.capacity(), 0);
            assert_eq!(lane.len(), 0);
        }

        let dummy_entry = LaneCarEntry {
            crashed: false,
            entity: Entity::from_bits(1),
            progress: 100.0,
            front_progress: 124.0,
            rear_progress: 76.0,
            current_speed: 240.0,
            cruise_speed: 240.0,
            committed_to_cross: false,
            stopping_for_signal: false,
        };
        scratch.lanes[0].reserve(10);
        let cap_before = scratch.lanes[0].capacity();
        assert!(cap_before >= 10);
        scratch.lanes[0].push(dummy_entry);
        assert_eq!(scratch.lanes[0].len(), 1);

        scratch.clear();
        assert_eq!(scratch.lanes[0].len(), 0);
        assert_eq!(scratch.lanes[0].capacity(), cap_before);

        // 2. Verify in Bevy ECS: Local<LaneCarScratch> retains capacity across system invocations
        #[derive(Resource, Default)]
        struct CapacityLog {
            frame_capacities: Vec<usize>,
        }

        fn check_scratch_system(mut scratch: Local<LaneCarScratch>, mut log: ResMut<CapacityLog>) {
            scratch.clear();
            if log.frame_capacities.is_empty() {
                // First frame: pre-populate / reserve capacity
                scratch.lanes[0].reserve(20);
            }
            log.frame_capacities.push(scratch.lanes[0].capacity());
        }

        let mut app = App::new();
        app.init_resource::<CapacityLog>();
        app.add_systems(Update, check_scratch_system);

        app.update(); // Frame 1
        app.update(); // Frame 2
        app.update(); // Frame 3

        let log = app.world().resource::<CapacityLog>();
        assert_eq!(log.frame_capacities.len(), 3);
        assert!(log.frame_capacities[0] >= 20);
        assert_eq!(log.frame_capacities[1], log.frame_capacities[0]);
        assert_eq!(log.frame_capacities[2], log.frame_capacities[0]);
    }

    #[test]
    fn test_compute_desired_speed_most_restrictive() {
        let decel = CAR_COMFORTABLE_DECELERATION; // 300.0
        let cruise = 240.0;

        // Pure function checks:
        // 1. Cruise speed only
        assert_eq!(
            compute_observed_speed_limit(cruise, None, None, decel),
            cruise
        );

        // 2. Leader gap restriction only (gap = 20.0 -> avail = 6.0 -> sqrt(2 * 300 * 6) = 60.0)
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(20.0), None, decel),
            60.0
        );

        // 3. Signal limit restriction only
        assert_eq!(
            compute_observed_speed_limit(cruise, None, Some(45.0), decel),
            45.0
        );

        // 4. Most restrictive between leader (60.0) and signal (45.0) -> 45.0
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(20.0), Some(45.0), decel),
            45.0
        );

        // 5. Most restrictive between leader (60.0) and signal (80.0) -> 60.0
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(20.0), Some(80.0), decel),
            60.0
        );

        // Bevy App test:
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();
        let stop_target_p = stop_line_progress(TrafficLane::Eastbound, &geom);

        let mut app = App::new();
        app.init_resource::<Time>();
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(
                DRIVER_REACTION_SECS + 0.05,
            ));
        }
        app.insert_resource(geom.clone());
        app.insert_resource(TrafficSignal::new(TrafficLightState::Red, 5.0));

        // Lead car positioned 24.0 units before the stop target
        // Safe speed = sqrt(2 * 300 * 24) = 120.0
        let lead_p = stop_target_p - 24.0;
        let lead_x = -geom.world_width / 2.0 - SPAWN_MARGIN + lead_p;
        let car_lead = app
            .world_mut()
            .spawn((
                Car::new(cruise, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(lead_x, east_y)),
                Heading::ZERO,
                CarMotion::new(cruise),
            ))
            .id();

        // Follower positioned behind lead car with gap = 20.0
        // Follower center = lead center - CAR_LENGTH - gap = lead center - 48.0 - 20.0 = lead center - 68.0
        // Available follow distance = 20.0 - 14.0 = 6.0
        // Safe follow speed = sqrt(2 * 300 * 6) = 60.0
        // Distance to stop line for follower is 24.0 + 68.0 = 92.0 -> safe signal speed ~ 234.9
        // Most restrictive must be leader follow speed (60.0)
        let follower_x = lead_x - CAR_LENGTH - 20.0;
        let car_follower = app
            .world_mut()
            .spawn((
                Car::new(cruise, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(follower_x, east_y)),
                Heading::ZERO,
                CarMotion::new(cruise),
            ))
            .id();

        app.add_systems(Update, plan_car_motion);
        app.update();

        let motion_lead = app.world().get::<CarMotion>(car_lead).unwrap();
        let motion_follower = app.world().get::<CarMotion>(car_follower).unwrap();

        assert_eq!(motion_lead.desired_speed, 120.0);
        assert_eq!(motion_follower.desired_speed, 60.0);
    }

    #[test]
    fn test_safe_stopping_speed_and_gap_clamping() {
        let a = CAR_COMFORTABLE_DECELERATION; // 300.0

        // v_safe = sqrt(2 * a * d)
        assert_eq!(safe_stopping_speed(6.0, a), 60.0);
        assert_eq!(safe_stopping_speed(24.0, a), 120.0);
        assert_eq!(safe_stopping_speed(0.0, a), 0.0);
        assert_eq!(safe_stopping_speed(-10.0, a), 0.0);
        assert_eq!(safe_stopping_speed(f32::NAN, a), 0.0);
        assert_eq!(safe_stopping_speed(f32::NEG_INFINITY, a), 0.0);
        assert_eq!(safe_stopping_speed(10.0, 0.0), 0.0);
        assert_eq!(safe_stopping_speed(10.0, -100.0), 0.0);
        assert_eq!(safe_stopping_speed(10.0, f32::NAN), 0.0);

        // Clamping invalid / overlapping gaps in compute_observed_speed_limit
        let cruise = 240.0;

        // Overlapping gap (negative) clamps to 0.0 -> available_d = 0.0 -> follow speed 0.0
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(-5.0), None, a),
            0.0
        );
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(0.0), None, a),
            0.0
        );
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(f32::NAN), None, a),
            0.0
        );

        // Gap equal to or smaller than CAR_MIN_FOLLOW_GAP (14.0) gives available_d = 0.0 -> 0.0
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(10.0), None, a),
            0.0
        );
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(14.0), None, a),
            0.0
        );

        // Valid positive gap larger than CAR_MIN_FOLLOW_GAP: gap = 20.0 -> d = 6.0 -> 60.0
        assert_eq!(
            compute_observed_speed_limit(cruise, Some(20.0), None, a),
            60.0
        );
    }

    #[test]
    fn test_driver_reaction_stricter_limit_delays_and_applies() {
        let mut motion = CarMotion::new(240.0);
        assert_eq!(motion.perceived_speed_limit, 240.0);
        assert_eq!(motion.pending_speed_limit, 240.0);
        assert_eq!(motion.reaction_remaining, 0.0);
        assert_eq!(motion.desired_speed, 240.0);

        // Frame 1: Stricter limit observed (100.0 < 240.0), dt = 0.1s
        // Timer starts at 0.35s and ticks down by 0.1s to 0.25s
        motion.update_driver_reaction(100.0, 0.1);
        assert!((motion.reaction_remaining - 0.25).abs() < 1e-4);
        assert_eq!(motion.pending_speed_limit, 100.0);
        assert_eq!(motion.perceived_speed_limit, 240.0);
        assert_eq!(motion.desired_speed, 240.0);

        // Frame 2: Even stricter limit observed (50.0 < 100.0), dt = 0.1s
        // Retains strictest pending limit without restarting timer: 0.25 - 0.1 = 0.15s
        motion.update_driver_reaction(50.0, 0.1);
        assert!((motion.reaction_remaining - 0.15).abs() < 1e-4);
        assert_eq!(motion.pending_speed_limit, 50.0);
        assert_eq!(motion.perceived_speed_limit, 240.0);
        assert_eq!(motion.desired_speed, 240.0);

        // Frame 3: Less strict than pending, but still stricter than perceived (80.0), dt = 0.1s
        // Retains 50.0 (strictest pending) without restarting timer: 0.15 - 0.1 = 0.05s
        motion.update_driver_reaction(80.0, 0.1);
        assert!((motion.reaction_remaining - 0.05).abs() < 1e-4);
        assert_eq!(motion.pending_speed_limit, 50.0);
        assert_eq!(motion.perceived_speed_limit, 240.0);
        assert_eq!(motion.desired_speed, 240.0);

        // Frame 4: Remaining 0.05s expires (dt = 0.06s)
        // Timer expires, pending limit applies to perceived and desired speed!
        motion.update_driver_reaction(50.0, 0.06);
        assert_eq!(motion.reaction_remaining, 0.0);
        assert_eq!(motion.pending_speed_limit, 50.0);
        assert_eq!(motion.perceived_speed_limit, 50.0);
        assert_eq!(motion.desired_speed, 50.0);

        // Frame 5: Relaxed limit observed (e.g. green light / leader accelerates: 240.0 >= 50.0)
        // Applies immediately without reaction delay!
        motion.update_driver_reaction(240.0, 0.1);
        assert_eq!(motion.reaction_remaining, 0.0);
        assert_eq!(motion.pending_speed_limit, 240.0);
        assert_eq!(motion.perceived_speed_limit, 240.0);
        assert_eq!(motion.desired_speed, 240.0);
    }

    #[test]
    fn test_driver_reaction_in_plan_car_motion() {
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();
        let stop_target_p = stop_line_progress(TrafficLane::Eastbound, &geom);

        let mut app = App::new();
        app.init_resource::<Time>();
        app.insert_resource(geom.clone());
        app.insert_resource(TrafficSignal::new(TrafficLightState::Red, 5.0));

        let lead_p = stop_target_p - 24.0;
        let lead_x = -geom.world_width / 2.0 - SPAWN_MARGIN + lead_p;
        let car = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(lead_x, east_y)),
                Heading::ZERO,
                CarMotion::new(240.0),
            ))
            .id();

        app.add_systems(Update, plan_car_motion);

        // Frame 1: advance by 0.1s (< 0.35s DRIVER_REACTION_SECS)
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.1));
        }
        app.update();

        let motion = app.world().get::<CarMotion>(car).unwrap();
        // Perceived and desired speed must hold old cruise speed (240.0) while reacting
        assert_eq!(motion.desired_speed, 240.0);
        assert_eq!(motion.perceived_speed_limit, 240.0);
        assert!((motion.reaction_remaining - 0.25).abs() < 1e-4);

        // Frame 2: advance by remaining 0.3s (0.1 + 0.3 = 0.4 > 0.35s)
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.3));
        }
        app.update();

        let motion = app.world().get::<CarMotion>(car).unwrap();
        // Reaction delay elapsed; signal stop limit (120.0) is applied
        assert_eq!(motion.desired_speed, 120.0);
        assert_eq!(motion.perceived_speed_limit, 120.0);
        assert_eq!(motion.reaction_remaining, 0.0);

        // Frame 3: Signal turns green
        {
            let mut signal = app.world_mut().resource_mut::<TrafficSignal>();
            signal.state = TrafficLightState::Green;
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.1));
        }
        app.update();

        let motion = app.world().get::<CarMotion>(car).unwrap();
        // Relaxed limit applies immediately!
        assert_eq!(motion.desired_speed, 240.0);
        assert_eq!(motion.perceived_speed_limit, 240.0);
        assert_eq!(motion.reaction_remaining, 0.0);
    }

    #[test]
    fn test_emergency_deceleration_activation_and_finite_step() {
        let dt = 0.1;
        // Case 1: Driver hasn't reacted yet -> emergency_braking remains false
        let mut motion = CarMotion::new(240.0);
        motion.update_driver_reaction(0.0, dt); // Reaction started, reaction_remaining = 0.25 > 0
        assert!(!motion.emergency_braking);
        assert_eq!(motion.perceived_speed_limit, 240.0);

        // Case 2: Reaction delay elapses, reacted safe speed (0.0) is below comfortable floor:
        // current_speed = 240.0, comfortable_floor = 240.0 - 300.0 * 0.3 = 150.0.
        // 0.0 < 150.0 -> emergency_braking becomes true!
        motion.update_driver_reaction(0.0, 0.3);
        assert_eq!(motion.reaction_remaining, 0.0);
        assert_eq!(motion.desired_speed, 0.0);
        assert!(motion.emergency_braking);

        // Stepping uses CAR_EMERGENCY_DECELERATION (520.0):
        // finite reduction: 240.0 - 520.0 * 0.1 = 188.0
        let new_speed = motion.step(0.1);
        assert_eq!(new_speed, 188.0);
        assert_eq!(motion.current_speed, 188.0);

        // Case 3: When desired speed is reachable via comfortable braking:
        // current_speed = 60.0, comfortable_floor for dt=0.1 is 60.0 - 300 * 0.1 = 30.0.
        // desired_speed = 40.0 >= 30.0 -> emergency_braking is false!
        let mut motion_comfort = CarMotion::new(60.0).with_speeds(60.0, 60.0);
        // Simulate reaction timer starting and immediately elapsing
        motion_comfort.reaction_remaining = 0.05;
        motion_comfort.pending_speed_limit = 40.0;
        motion_comfort.update_driver_reaction(40.0, 0.1);
        assert_eq!(motion_comfort.reaction_remaining, 0.0);
        assert_eq!(motion_comfort.desired_speed, 40.0);
        assert!(!motion_comfort.emergency_braking);
        let speed = motion_comfort.step(0.1);
        // Comfortable braking reaches desired speed 40.0 without emergency braking:
        assert_eq!(speed, 40.0);
    }

    #[test]
    fn test_three_car_queue() {
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();
        let stop_target_p = stop_line_progress(TrafficLane::Eastbound, &geom);
        let start_x = -geom.world_width / 2.0 - SPAWN_MARGIN;

        let mut app = App::new();
        app.init_resource::<Time>();
        app.insert_resource(geom.clone());
        app.insert_resource(TrafficSignal::new(TrafficLightState::Red, 10.0));
        app.add_systems(
            Update,
            (plan_car_motion, apply_car_acceleration, car_movement).chain(),
        );

        let cruise = 240.0;
        let gap = 200.0; // Adequate following distance (~194 u safe gap at 240 u/s)

        // Lead car starts with room to react and brake comfortably before red stop line
        let lead_p = stop_target_p - 200.0;
        let mid_p = lead_p - CAR_LENGTH - gap;
        let rear_p = mid_p - CAR_LENGTH - gap;

        let car_lead = app
            .world_mut()
            .spawn((
                Car::new(cruise, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(start_x + lead_p, east_y)),
                Heading::ZERO,
                CarMotion::new(cruise),
            ))
            .id();

        let car_mid = app
            .world_mut()
            .spawn((
                Car::new(cruise, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(start_x + mid_p, east_y)),
                Heading::ZERO,
                CarMotion::new(cruise),
            ))
            .id();

        let car_rear = app
            .world_mut()
            .spawn((
                Car::new(cruise, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(start_x + rear_p, east_y)),
                Heading::ZERO,
                CarMotion::new(cruise),
            ))
            .id();

        // Phase 1: Vehicles approach red light and react in order.
        let dt = 0.05;
        let mut lead_reacted_step = None;
        let mut mid_reacted_step = None;
        let mut rear_reacted_step = None;

        for step in 0..160 {
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(std::time::Duration::from_secs_f32(dt));
            }
            app.update();

            let m_lead = app.world().get::<CarMotion>(car_lead).unwrap();
            let m_mid = app.world().get::<CarMotion>(car_mid).unwrap();
            let m_rear = app.world().get::<CarMotion>(car_rear).unwrap();
            if lead_reacted_step.is_none() && m_lead.current_speed < cruise {
                lead_reacted_step = Some(step);
            }
            if mid_reacted_step.is_none() && m_mid.current_speed < cruise {
                mid_reacted_step = Some(step);
            }
            if rear_reacted_step.is_none() && m_rear.current_speed < cruise {
                rear_reacted_step = Some(step);
            }
        }

        // Assert followers react in order
        assert!(lead_reacted_step.is_some(), "Lead car must begin braking");
        assert!(mid_reacted_step.is_some(), "Mid car must begin braking");
        assert!(rear_reacted_step.is_some(), "Rear car must begin braking");
        let s_lead = lead_reacted_step.unwrap();
        let s_mid = mid_reacted_step.unwrap();
        let s_rear = rear_reacted_step.unwrap();
        assert!(
            s_lead < s_mid,
            "Lead car must brake before mid follower (lead: {s_lead}, mid: {s_mid})"
        );
        assert!(
            s_mid < s_rear,
            "Mid follower must brake before rear follower (mid: {s_mid}, rear: {s_rear})"
        );

        // Phase 2: All stop without overlap at adequate spacing
        let m_lead = app.world().get::<CarMotion>(car_lead).unwrap();
        let m_mid = app.world().get::<CarMotion>(car_mid).unwrap();
        let m_rear = app.world().get::<CarMotion>(car_rear).unwrap();
        assert_eq!(
            m_lead.current_speed, 0.0,
            "Lead car must come to complete stop"
        );
        assert_eq!(
            m_mid.current_speed, 0.0,
            "Mid car must come to complete stop"
        );
        assert_eq!(
            m_rear.current_speed, 0.0,
            "Rear car must come to complete stop"
        );

        let pos_lead = app.world().get::<GroundPos>(car_lead).unwrap().0;
        let pos_mid = app.world().get::<GroundPos>(car_mid).unwrap().0;
        let pos_rear = app.world().get::<GroundPos>(car_rear).unwrap().0;

        let car_comp = app.world().get::<Car>(car_lead).unwrap();
        let p_lead = lane_progress(TrafficLane::Eastbound, pos_lead, &geom);
        let p_mid = lane_progress(TrafficLane::Eastbound, pos_mid, &geom);
        let p_rear = lane_progress(TrafficLane::Eastbound, pos_rear, &geom);

        let gap_lead_mid = bumper_gap(
            car_comp.rear_progress(p_lead),
            car_comp.front_progress(p_mid),
        );
        let gap_mid_rear = bumper_gap(
            car_comp.rear_progress(p_mid),
            car_comp.front_progress(p_rear),
        );

        assert!(
            p_lead <= stop_target_p + STOP_LINE_BUFFER,
            "Lead car must stop at or before the stop line"
        );
        assert!(
            gap_lead_mid >= CAR_MIN_FOLLOW_GAP - 1.0,
            "Lead-Mid gap ({gap_lead_mid}) must be adequate without overlap"
        );
        assert!(
            gap_mid_rear >= CAR_MIN_FOLLOW_GAP - 1.0,
            "Mid-Rear gap ({gap_mid_rear}) must be adequate without overlap"
        );

        // Phase 3: Traffic light turns green, all cars accelerate after green
        {
            let mut signal = app.world_mut().resource_mut::<TrafficSignal>();
            signal.state = TrafficLightState::Green;
        }

        // Advance several seconds on green
        for _ in 0..60 {
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(std::time::Duration::from_secs_f32(dt));
            }
            app.update();
        }

        let m_lead_after = app.world().get::<CarMotion>(car_lead).unwrap();
        let m_mid_after = app.world().get::<CarMotion>(car_mid).unwrap();
        let m_rear_after = app.world().get::<CarMotion>(car_rear).unwrap();

        assert!(
            m_lead_after.current_speed > 0.0,
            "Lead car must accelerate after green"
        );
        assert!(
            m_mid_after.current_speed > 0.0,
            "Mid car must accelerate after green"
        );
        assert!(
            m_rear_after.current_speed > 0.0,
            "Rear car must accelerate after green"
        );
    }

    #[test]
    fn test_abrupt_stopped_leader_two_gaps() {
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();
        let start_x = -geom.world_width / 2.0 - SPAWN_MARGIN;
        let dt = 0.05;
        let cruise = CAR_SPEED; // 240.0

        // Sub-scenario helper: runs leader-follower simulation and returns:
        // (min_gap, final_follower_speed, emergency_braking_activated)
        let run_scenario = |initial_gap: f32| -> (f32, f32, bool) {
            let mut app = App::new();
            app.init_resource::<Time>();
            app.insert_resource(geom.clone());
            app.insert_resource(TrafficSignal::new(TrafficLightState::Green, 100.0));
            app.add_systems(
                Update,
                (plan_car_motion, apply_car_acceleration, car_movement).chain(),
            );

            let lead_p = 600.0;
            let follow_p = lead_p - CAR_LENGTH - initial_gap;

            // Stopped leader (stationary vehicle)
            let car_lead = app
                .world_mut()
                .spawn((
                    Car::new(0.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                    GroundPos(Vec2::new(start_x + lead_p, east_y)),
                    Heading::ZERO,
                    CarMotion::new(0.0),
                ))
                .id();

            // Follower cruising toward leader
            let car_follow = app
                .world_mut()
                .spawn((
                    Car::new(cruise, Vec2::X, CAR_LENGTH, CAR_WIDTH)
                        .with_lane(TrafficLane::Eastbound),
                    GroundPos(Vec2::new(start_x + follow_p, east_y)),
                    Heading::ZERO,
                    CarMotion::new(cruise),
                ))
                .id();

            let mut min_gap = f32::INFINITY;
            let mut emergency_activated = false;

            for _ in 0..100 {
                {
                    let mut time = app.world_mut().resource_mut::<Time>();
                    time.advance_by(std::time::Duration::from_secs_f32(dt));
                }
                app.update();

                let pos_lead = app.world().get::<GroundPos>(car_lead).unwrap().0;
                let pos_follow = app.world().get::<GroundPos>(car_follow).unwrap().0;
                let p_lead = lane_progress(TrafficLane::Eastbound, pos_lead, &geom);
                let p_follow = lane_progress(TrafficLane::Eastbound, pos_follow, &geom);
                let lead_rear = rear_progress(p_lead, CAR_LENGTH);
                let follow_front = front_progress(p_follow, CAR_LENGTH);
                let current_gap = bumper_gap(lead_rear, follow_front);

                if current_gap < min_gap {
                    min_gap = current_gap;
                }

                let m_follow = app.world().get::<CarMotion>(car_follow).unwrap();
                if m_follow.emergency_braking {
                    emergency_activated = true;
                }

                if m_follow.current_speed <= 0.0 || current_gap <= 0.0 {
                    break;
                }
            }

            let final_speed = app
                .world()
                .get::<CarMotion>(car_follow)
                .unwrap()
                .current_speed;
            (min_gap, final_speed, emergency_activated)
        };

        // 1. Adequate distance (250.0):
        // Follower approaches stopped leader from adequate distance.
        // Follower detects stopped leader, reacts, brakes, and comes to a complete stop
        // without contact (min_gap > 0.0) while preserving minimum follow spacing.
        let (min_gap_ok, final_speed_ok, _) = run_scenario(250.0);
        assert_eq!(final_speed_ok, 0.0, "Follower must come to complete stop");
        assert!(
            min_gap_ok > 0.0,
            "Adequate distance must avoid contact (min_gap: {min_gap_ok})"
        );
        assert!(
            min_gap_ok >= CAR_MIN_FOLLOW_GAP - 1.0,
            "Adequate distance must maintain safe follow gap (min_gap: {min_gap_ok})"
        );

        // 2. Insufficient distance (90.0):
        // Follower approaches stopped leader from insufficient distance.
        // Driver reaction delay postpones braking; once reacted, emergency braking activates,
        // but finite emergency deceleration is insufficient to prevent rear-end overlap (gap <= 0.0).
        let (min_gap_bad, _, emerg_bad) = run_scenario(90.0);
        assert!(
            emerg_bad,
            "Emergency braking must be triggered under abrupt insufficient distance"
        );
        assert!(
            min_gap_bad <= 0.0,
            "Insufficient distance must create overlap despite emergency braking (min_gap: {min_gap_bad})"
        );
    }

    #[test]
    fn test_detect_rear_end_crashes() {
        let mut app = App::new();
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();
        let west_y = geom.west_lane_y();

        #[derive(Resource, Default)]
        struct RecordedCrashes(Vec<RearEndCrashEvent>);

        app.init_resource::<RecordedCrashes>();
        app.add_observer(
            |event: On<RearEndCrashEvent>, mut records: ResMut<RecordedCrashes>| {
                records.0.push(*event);
            },
        );
        app.insert_resource(geom)
            .add_systems(Update, detect_rear_end_crashes);

        // Two cars in Eastbound lane:
        // Leader at x = 100.0, follower at x = 50.0.
        // Car length is 48.0 (half length 24.0).
        // Leader rear bumper = 100.0 - 24.0 = 76.0.
        // Follower front bumper = 50.0 + 24.0 = 74.0.
        // Gap = 76.0 - 74.0 = 2.0 > 0 (no collision).
        let leader = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(100.0, east_y)),
            ))
            .id();
        let follower = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(50.0, east_y)),
            ))
            .id();

        // Another car in Westbound lane with overlapping x coordinates (adjacent lane non-collision check)
        let _west_car = app
            .world_mut()
            .spawn((
                Car::new(240.0, -Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Westbound),
                GroundPos(Vec2::new(75.0, west_y)),
            ))
            .id();

        app.update();

        assert!(
            app.world().resource::<RecordedCrashes>().0.is_empty(),
            "Gap > 0 and adjacent lanes must not trigger rear-end collision"
        );

        // Move follower forward so front bumper penetrates leader rear bumper:
        // Follower at x = 55.0 -> front bumper = 79.0.
        // Gap = 76.0 - 79.0 = -3.0 <= 0 -> collision!
        *app.world_mut().get_mut::<GroundPos>(follower).unwrap() =
            GroundPos(Vec2::new(55.0, east_y));

        app.update();

        let recorded = app.world().resource::<RecordedCrashes>();
        assert_eq!(recorded.0.len(), 1, "Exactly one crash event expected");
        let crash = recorded.0[0];
        assert_eq!(crash.leader, leader);
        assert_eq!(crash.follower, follower);
        assert_eq!(crash.lane, TrafficLane::Eastbound);
        assert!(crash.gap <= 0.0);

        // Tick time on leader's crashed timer
        {
            let mut leader_crashed = app.world_mut().get_mut::<Crashed>(leader).unwrap();
            leader_crashed.timer.tick(std::time::Duration::from_secs(1));
            assert_eq!(leader_crashed.timer.elapsed_secs(), 1.0);
        }

        // Run next frame - already-crashed pair must NOT emit duplicate event or refresh timer
        app.update();
        let recorded = app.world().resource::<RecordedCrashes>();
        assert_eq!(
            recorded.0.len(),
            1,
            "Already crashed pair must not emit duplicate crash event"
        );
        let leader_crashed_after = app.world().get::<Crashed>(leader).unwrap();
        assert_eq!(
            leader_crashed_after.timer.elapsed_secs(),
            1.0,
            "Timer must not be refreshed or re-inserted for already crashed car"
        );
    }
    #[test]
    fn test_crashed_component_and_motion_zeroed_on_impact() {
        let mut app = App::new();
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();

        app.init_resource::<Time>()
            .insert_resource(geom)
            .add_systems(
                Update,
                (
                    plan_car_motion,
                    apply_car_acceleration,
                    car_movement,
                    detect_rear_end_crashes,
                )
                    .chain(),
            );

        let leader = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(240.0),
                GroundPos(Vec2::new(100.0, east_y)),
                Heading(0.0),
            ))
            .id();

        let follower = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(240.0),
                // Follower placed with front bumper penetrating leader rear bumper
                // Leader rear = 100 - 24 = 76. Follower front = 55 + 24 = 79.
                GroundPos(Vec2::new(55.0, east_y)),
                Heading(0.0),
            ))
            .id();

        // Run one frame with detect_rear_end_crashes
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();
        // 1. Both cars must be marked with Crashed component
        let leader_crashed = app.world().get::<Crashed>(leader);
        assert!(
            leader_crashed.is_some(),
            "Leader must have Crashed component"
        );
        assert_eq!(
            leader_crashed.unwrap().timer.duration().as_secs_f32(),
            CRASH_CLEANUP_SECS,
            "Cleanup timer must be set to CRASH_CLEANUP_SECS"
        );

        let follower_crashed = app.world().get::<Crashed>(follower);
        assert!(
            follower_crashed.is_some(),
            "Follower must have Crashed component"
        );
        assert_eq!(
            follower_crashed.unwrap().timer.duration().as_secs_f32(),
            CRASH_CLEANUP_SECS,
            "Cleanup timer must be set to CRASH_CLEANUP_SECS"
        );

        // 2. Both cars must have all motion speeds/limits set to zero
        let leader_car = app.world().get::<Car>(leader).unwrap();
        let leader_motion = app.world().get::<CarMotion>(leader).unwrap();
        assert_eq!(leader_car.speed, 0.0);
        assert_eq!(leader_motion.current_speed, 0.0);
        assert_eq!(leader_motion.desired_speed, 0.0);
        assert_eq!(leader_motion.perceived_speed_limit, 0.0);
        assert_eq!(leader_motion.pending_speed_limit, 0.0);
        assert_eq!(leader_motion.cruise_speed, 0.0);

        let follower_car = app.world().get::<Car>(follower).unwrap();
        let follower_motion = app.world().get::<CarMotion>(follower).unwrap();
        assert_eq!(follower_car.speed, 0.0);
        assert_eq!(follower_motion.current_speed, 0.0);
        assert_eq!(follower_motion.desired_speed, 0.0);
        assert_eq!(follower_motion.perceived_speed_limit, 0.0);
        assert_eq!(follower_motion.pending_speed_limit, 0.0);
        assert_eq!(follower_motion.cruise_speed, 0.0);

        // 3. Crashed cars remain physical obstacles:
        // Spawn car 3 approaching from behind. In plan_car_motion, car 3 must detect
        // the crashed follower as leader ahead and target safe stopping speed.
        let third_car = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(240.0),
                GroundPos(Vec2::new(-50.0, east_y)),
                Heading(0.0),
            ))
            .id();

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();
        let third_motion = app.world().get::<CarMotion>(third_car).unwrap();
        // Follower moved 12.0px during the 50ms crash frame to x = 67.0.
        // Follower rear bumper is 67.0 - 24.0 = 43.0.
        // Third car front bumper is -50.0 + 24.0 = -26.0.
        // Gap is 43.0 - (-26.0) = 69.0.
        assert!(
            third_motion.leader_gap.is_some(),
            "Third car must detect leader ahead"
        );
        let gap = third_motion.leader_gap.unwrap();
        assert!(
            (gap - 69.0).abs() < 1e-3,
            "Leader gap to crashed car must be ~69.0, was {gap}"
        );
        assert!(
            third_motion.pending_speed_limit < 240.0,
            "Third car pending speed limit must be reduced to stop behind crashed vehicle"
        );

        // After reaction delay (DRIVER_REACTION_SECS = 0.35s), desired speed drops
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(350));
        app.update();
        let third_motion_after = app.world().get::<CarMotion>(third_car).unwrap();
        assert!(
            third_motion_after.desired_speed < 240.0,
            "Third car desired speed must be reduced to stop behind crashed vehicle"
        );

        // 4. Over multiple frames, crashed vehicles do not move or accelerate
        let leader_pos_before = app.world().get::<GroundPos>(leader).unwrap().0;
        let follower_pos_before = app.world().get::<GroundPos>(follower).unwrap().0;

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();
        let leader_pos_after = app.world().get::<GroundPos>(leader).unwrap().0;
        let follower_pos_after = app.world().get::<GroundPos>(follower).unwrap().0;
        assert_eq!(
            leader_pos_before, leader_pos_after,
            "Crashed leader must not move"
        );
        assert_eq!(
            follower_pos_before, follower_pos_after,
            "Crashed follower must not move"
        );
    }

    #[test]
    fn test_rear_end_crash_full_lifecycle_and_deferral() {
        let mut app = App::new();
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();
        let west_y = geom.west_lane_y();

        #[derive(Resource, Default)]
        struct CrashCounter(usize);

        app.init_resource::<Time>()
            .init_resource::<CrashCounter>()
            .init_resource::<Assets<Mesh>>()
            .insert_resource(SpawnTimer::new(1000.0))
            .insert_resource(geom.clone())
            .add_observer(
                |_event: On<RearEndCrashEvent>, mut counter: ResMut<CrashCounter>| {
                    counter.0 += 1;
                },
            )
            .add_systems(
                Update,
                (
                    spawn_cars,
                    plan_car_motion,
                    apply_car_acceleration,
                    car_movement,
                    detect_rear_end_crashes,
                    tick_crash_cleanup,
                    car_cleanup,
                )
                    .chain(),
            );

        // 1. Adjacent-lane non-collision: Westbound car overlapping Eastbound car in x
        let west_car = app
            .world_mut()
            .spawn((
                Car::new(200.0, -Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Westbound),
                CarMotion::new(200.0),
                GroundPos(Vec2::new(100.0, west_y)),
                Heading(std::f32::consts::PI),
            ))
            .id();

        let east_leader = app
            .world_mut()
            .spawn((
                Car::new(0.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(0.0),
                GroundPos(Vec2::new(100.0, east_y)),
                Heading(0.0),
            ))
            .id();

        // Follower trailing leader with safe gap (gap = (100-24) - (40+24) = 76 - 64 = 12 > 0)
        let east_follower = app
            .world_mut()
            .spawn((
                Car::new(240.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(240.0),
                GroundPos(Vec2::new(40.0, east_y)),
                Heading(0.0),
            ))
            .id();

        // Frame 1: Advance by 10ms - gap > 0, no crashes
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(10));
        app.update();
        assert_eq!(
            app.world().resource::<CrashCounter>().0,
            0,
            "No crash before contact"
        );
        assert!(app.world().get::<Crashed>(east_leader).is_none());
        assert!(app.world().get::<Crashed>(east_follower).is_none());
        assert!(app.world().get::<Crashed>(west_car).is_none());

        // 2. One impact transition: Move follower to penetrate leader bumper
        *app.world_mut().get_mut::<GroundPos>(east_follower).unwrap() =
            GroundPos(Vec2::new(55.0, east_y));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();

        assert_eq!(
            app.world().resource::<CrashCounter>().0,
            1,
            "Exactly one impact event"
        );
        assert!(
            app.world().get::<Crashed>(east_leader).is_some(),
            "Leader must transition to Crashed"
        );
        assert!(
            app.world().get::<Crashed>(east_follower).is_some(),
            "Follower must transition to Crashed"
        );
        assert!(
            app.world().get::<Crashed>(west_car).is_none(),
            "Adjacent lane car must not crash"
        );

        // 3. Both cars stopping: speeds and motion set to zero
        assert_eq!(app.world().get::<Car>(east_leader).unwrap().speed, 0.0);
        assert_eq!(
            app.world()
                .get::<CarMotion>(east_leader)
                .unwrap()
                .current_speed,
            0.0
        );
        assert_eq!(app.world().get::<Car>(east_follower).unwrap().speed, 0.0);
        assert_eq!(
            app.world()
                .get::<CarMotion>(east_follower)
                .unwrap()
                .current_speed,
            0.0
        );

        // 4. No duplicate impact on next frame
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();
        assert_eq!(
            app.world().resource::<CrashCounter>().0,
            1,
            "No duplicate impact on next frame"
        );

        // 5. Timed cleanup: after 3.0s total elapsed crash time, both cars despawn
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(2800));
        app.update();
        assert!(
            app.world().get_entity(east_leader).is_ok(),
            "Leader remains before 3.0s"
        );
        assert!(
            app.world().get_entity(east_follower).is_ok(),
            "Follower remains before 3.0s"
        );

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(200));
        app.update();
        assert!(
            app.world().get_entity(east_leader).is_err(),
            "Leader despawned after 3.0s"
        );
        assert!(
            app.world().get_entity(east_follower).is_err(),
            "Follower despawned after 3.0s"
        );

        // 6. Blocked-spawn deferral
        let east_spawn = TrafficLane::Eastbound.spawn_position(&geom);
        let blocker = app
            .world_mut()
            .spawn((
                Car::new(0.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(0.0),
                GroundPos(east_spawn),
                Heading(0.0),
            ))
            .id();

        // Reset spawn timer and trigger spawn attempt via advance_by
        {
            let mut spawn_timer = app.world_mut().resource_mut::<SpawnTimer>();
            spawn_timer
                .timer
                .set_duration(std::time::Duration::from_secs_f32(CAR_SPAWN_INTERVAL_SECS));
            spawn_timer.reset();
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(CAR_SPAWN_INTERVAL_SECS));
        app.update();

        // Spawn footprint occupied by blocker -> spawn deferred
        assert_eq!(
            app.world_mut().query::<&Car>().iter(app.world()).count(),
            2, // west_car + blocker
            "Car must not spawn into occupied footprint"
        );
        let spawn_timer = app.world().resource::<SpawnTimer>();
        assert!(spawn_timer.deferred, "Spawn should be marked deferred");
        assert_eq!(spawn_timer.current_lane(), TrafficLane::Eastbound);

        // Move blocker forward to clear spawn footprint
        *app.world_mut().get_mut::<GroundPos>(blocker).unwrap() =
            GroundPos(east_spawn + Vec2::new(100.0, 0.0));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();

        // Now deferred car should spawn
        assert_eq!(
            app.world_mut().query::<&Car>().iter(app.world()).count(),
            3, // west_car + blocker + newly spawned car
            "Deferred car must spawn once clearance is available"
        );
        let spawn_timer_after = app.world().resource::<SpawnTimer>();
        assert!(
            !spawn_timer_after.deferred,
            "Deferred flag must clear after spawn"
        );
        assert_eq!(
            spawn_timer_after.current_lane(),
            TrafficLane::Westbound,
            "Spawn lane should advance to next lane"
        );
    }

    #[test]
    fn test_car_motion_is_braking_and_visual_state() {
        // 1. Cruising normally at constant cruise speed -> not braking, visual state Normal
        let cruising = CarMotion::new(240.0);
        assert!(!cruising.is_braking());
        assert_eq!(cruising.visual_state(false), CarVisualState::Normal);
        assert_eq!(cruising.visual_state(true), CarVisualState::Crashed);

        // 2. Actively decelerating (desired < current) -> braking, visual state Braking
        let decelerating = CarMotion::new(240.0).with_speeds(240.0, 100.0);
        assert!(decelerating.is_braking());
        assert_eq!(decelerating.visual_state(false), CarVisualState::Braking);
        assert_eq!(decelerating.visual_state(true), CarVisualState::Crashed);

        // 3. Emergency braking engaged -> braking, visual state Braking
        let emergency = CarMotion::new(240.0).with_emergency_braking(true);
        assert!(emergency.is_braking());
        assert_eq!(emergency.visual_state(false), CarVisualState::Braking);
        assert_eq!(emergency.visual_state(true), CarVisualState::Crashed);

        // 4. Stopped holding brakes (current == 0, desired == 0, cruise > 0) -> braking
        let stopped_at_light = CarMotion::new(240.0).with_speeds(0.0, 0.0);
        assert!(stopped_at_light.is_braking());
        assert_eq!(
            stopped_at_light.visual_state(false),
            CarVisualState::Braking
        );
        assert_eq!(stopped_at_light.visual_state(true), CarVisualState::Crashed);

        // 5. Accelerating on green from stop (current == 0, desired == 240) -> not braking
        let accelerating = CarMotion::new(240.0).with_speeds(0.0, 240.0);
        assert!(!accelerating.is_braking());
        assert_eq!(accelerating.visual_state(false), CarVisualState::Normal);
        assert_eq!(accelerating.visual_state(true), CarVisualState::Crashed);

        // 6. Permanent stop after crash (stop_motion clears cruise_speed to 0) -> not braking
        let mut wrecked = CarMotion::new(240.0);
        wrecked.stop_motion();
        assert!(!wrecked.is_braking());
        assert_eq!(wrecked.visual_state(true), CarVisualState::Crashed);
    }

    #[test]
    fn test_traffic_systems_explicit_chain_order() {
        use bevy::state::app::StatesPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<RoadGeometry>()
            .init_state::<AppState>()
            .add_plugins(TrafficPlugin);

        // AppState defaults to InGame, so traffic systems are registered in Update
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );
        // Run update frames to verify the chained systems execute without conflict
        app.update();
    }

    #[test]
    fn test_traffic_chain_gated_by_in_game_freezes_in_non_game_states() {
        use bevy::state::app::StatesPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<RoadGeometry>()
            .init_state::<AppState>()
            .add_plugins(TrafficPlugin);

        // Transition to GameOver
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::GameOver);
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::GameOver
        );

        // Setup moving car with reaction timer in flight
        let moving_car = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(0.0, 0.0)),
                Heading(0.0),
                CarMotion::new(200.0),
            ))
            .id();
        {
            let mut motion = app.world_mut().get_mut::<CarMotion>(moving_car).unwrap();
            motion.reaction_remaining = 0.20;
            motion.pending_speed_limit = 0.0;
        }

        // Setup crashed car with cleanup timer
        let crashed_car = app
            .world_mut()
            .spawn((
                Car::new(0.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                GroundPos(Vec2::new(100.0, 0.0)),
                Heading(0.0),
                CarMotion::new(0.0),
                Crashed::default(),
            ))
            .id();

        let initial_signal_elapsed = app.world().resource::<TrafficSignal>().timer.elapsed_secs();
        let initial_signal_state = app.world().resource::<TrafficSignal>().state;

        // Advance time by 5.0 seconds in GameOver state
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(5));
        app.update();

        // 1. Verify signal phase and timer are frozen
        let signal = app.world().resource::<TrafficSignal>();
        assert_eq!(signal.state, initial_signal_state);
        assert_eq!(signal.timer.elapsed_secs(), initial_signal_elapsed);

        // 2. Verify driver reaction timer and movement are frozen
        let motion = app.world().get::<CarMotion>(moving_car).unwrap();
        assert_eq!(motion.reaction_remaining, 0.20);
        assert_eq!(motion.current_speed, 200.0);
        let pos = app.world().get::<GroundPos>(moving_car).unwrap();
        assert_eq!(pos.0, Vec2::new(0.0, 0.0));

        // 3. Verify crash cleanup timer is frozen and car not despawned
        assert!(app.world().get_entity(crashed_car).is_ok());
        let crashed = app.world().get::<Crashed>(crashed_car).unwrap();
        assert_eq!(crashed.timer.elapsed_secs(), 0.0);

        // Now resume InGame state
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );

        // Advance time by 1.0 second in InGame state
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.update();

        // 4. Verify systems actively advance in InGame state
        let signal_resumed = app.world().resource::<TrafficSignal>();
        assert!(signal_resumed.timer.elapsed_secs() > initial_signal_elapsed);

        let pos_resumed = app.world().get::<GroundPos>(moving_car).unwrap();
        assert!(pos_resumed.0.x > 0.0, "Car must have moved forward");

        let crashed_resumed = app.world().get::<Crashed>(crashed_car).unwrap();
        assert!(
            crashed_resumed.timer.elapsed_secs() > 0.0,
            "Crash timer must advance"
        );
    }

    #[test]
    fn test_integrated_deterministic_scenario_complete_cycle() {
        let mut app = App::new();
        let geom = RoadGeometry::default();
        let east_y = geom.east_lane_y();

        #[derive(Resource, Default)]
        struct CrashCounter(usize);

        app.init_resource::<Time>()
            .init_resource::<CrashCounter>()
            .init_resource::<Assets<Mesh>>()
            .insert_resource(SpawnTimer::new(10000.0)) // disable random spawns during scenario
            .insert_resource(geom.clone())
            .insert_resource(TrafficSignal::new(TrafficLightState::Green, 0.5)) // 0.5s green remaining
            .add_observer(
                |_event: On<RearEndCrashEvent>, mut counter: ResMut<CrashCounter>| {
                    counter.0 += 1;
                },
            )
            .add_systems(
                Update,
                (
                    tick_traffic_signal,
                    spawn_cars,
                    plan_car_motion,
                    apply_car_acceleration,
                    car_movement,
                    detect_rear_end_crashes,
                    tick_crash_cleanup,
                    car_cleanup,
                )
                    .chain(),
            );

        let stop_pos = geom.stop_line_position(TrafficLane::Eastbound);
        let stop_line = lane_progress(TrafficLane::Eastbound, stop_pos, &geom);

        // 1. Cars enter on green: Leader at -400.0, Follower at -650.0
        let leader = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(200.0),
                GroundPos(Vec2::new(-400.0, east_y)),
                Heading::ZERO,
            ))
            .id();

        let follower = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(200.0),
                GroundPos(Vec2::new(-650.0, east_y)),
                Heading::ZERO,
            ))
            .id();

        let dt = std::time::Duration::from_millis(50); // 50ms per step

        // Step through the 0.5s Green phase (10 steps)
        for _ in 0..10 {
            app.world_mut().resource_mut::<Time>().advance_by(dt);
            app.update();
        }

        // Verify cars moved forward on green
        let lead_pos = app.world().get::<GroundPos>(leader).unwrap().0;
        let fol_pos = app.world().get::<GroundPos>(follower).unwrap().0;
        assert!(lead_pos.x > -400.0, "Leader moved forward on green");
        assert!(fol_pos.x > -650.0, "Follower moved forward on green");
        assert_eq!(
            app.world().resource::<TrafficSignal>().state,
            TrafficLightState::Yellow
        );

        // Step through Yellow (1.5s = 30 steps) and into Red (4.5s = 90 steps, total 120 steps = 6.0s)
        // Cars should decelerate smoothly and queue on red before the stop line
        for _ in 0..120 {
            app.world_mut().resource_mut::<Time>().advance_by(dt);
            app.update();
        }

        // Verify: Both cars are queued on red
        assert_eq!(
            app.world().resource::<TrafficSignal>().state,
            TrafficLightState::Red
        );
        let lead_motion = app.world().get::<CarMotion>(leader).unwrap();
        let fol_motion = app.world().get::<CarMotion>(follower).unwrap();
        let lead_pos = app.world().get::<GroundPos>(leader).unwrap().0;
        let fol_pos = app.world().get::<GroundPos>(follower).unwrap().0;
        let lead_car = app.world().get::<Car>(leader).unwrap();
        let fol_car = app.world().get::<Car>(follower).unwrap();

        assert_eq!(
            lead_motion.current_speed, 0.0,
            "Leader must come to full stop on red"
        );
        assert_eq!(
            fol_motion.current_speed, 0.0,
            "Follower must come to full stop in queue"
        );

        // Verify: Neither car crossed the stop line
        let lead_center_p = lane_progress(TrafficLane::Eastbound, lead_pos, &geom);
        let fol_center_p = lane_progress(TrafficLane::Eastbound, fol_pos, &geom);
        assert!(
            lead_car.front_progress(lead_center_p) <= stop_line,
            "Leader front progress ({}) must not cross stop line ({})",
            lead_car.front_progress(lead_center_p),
            stop_line
        );
        assert!(
            fol_car.front_progress(fol_center_p) <= lead_car.rear_progress(lead_center_p),
            "Follower must be behind leader"
        );
        let q_gap = bumper_gap(
            lead_car.rear_progress(lead_center_p),
            fol_car.front_progress(fol_center_p),
        );
        assert!(
            q_gap >= 0.0,
            "Queued cars must not overlap (gap = {})",
            q_gap
        );

        // Step remaining 10 steps (0.5s) to complete Red (total 5.0s red) and transition to Green
        for _ in 0..10 {
            app.world_mut().resource_mut::<Time>().advance_by(dt);
            app.update();
        }

        // Verify: Signal has returned to Green (0.5s + 1.5s yellow + 5.0s red = 7.0s total)
        assert_eq!(
            app.world().resource::<TrafficSignal>().state,
            TrafficLightState::Green
        );

        // Step through restart on Green (1.5s = 30 steps)
        for _ in 0..30 {
            app.world_mut().resource_mut::<Time>().advance_by(dt);
            app.update();
        }

        // Verify: Both cars restarted and accelerated forward on Green
        let lead_motion_resumed = app.world().get::<CarMotion>(leader).unwrap();
        let fol_motion_resumed = app.world().get::<CarMotion>(follower).unwrap();
        assert!(
            lead_motion_resumed.current_speed > 0.0,
            "Leader must accelerate forward on green"
        );
        assert!(
            fol_motion_resumed.current_speed > 0.0,
            "Follower must accelerate forward in wake"
        );

        // Despawn earlier cars to isolate crash test
        app.world_mut().despawn(leader);
        app.world_mut().despawn(follower);

        // Part 2: One deliberately short-gap follower crashes after an abrupt stop
        // Lead car moving at speed 200.0, follower trailing close behind at speed 200.0
        // Lead car center at x = 0.0, rear bumper = -24.0
        let lead_moving = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(200.0),
                GroundPos(Vec2::new(0.0, east_y)),
                Heading::ZERO,
            ))
            .id();

        // Short-gap follower trailing by only 6.0 units bumper gap, travelling at 200.0
        // Leader rear bumper = 0.0 - 24.0 = -24.0
        // Follower front bumper = -24.0 - 6.0 = -30.0
        // Follower center = -30.0 - 24.0 = -54.0
        let crash_follower = app
            .world_mut()
            .spawn((
                Car::new(200.0, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_lane(TrafficLane::Eastbound),
                CarMotion::new(200.0),
                GroundPos(Vec2::new(-54.0, east_y)),
                Heading::ZERO,
            ))
            .id();

        assert_eq!(app.world().resource::<CrashCounter>().0, 0);

        // Step 1 frame: both move forward together at speed 200.0 without crashing
        app.world_mut().resource_mut::<Time>().advance_by(dt);
        app.update();
        assert_eq!(
            app.world().resource::<CrashCounter>().0,
            0,
            "No crash while travelling at matched speeds"
        );

        // Lead car stops abruptly mid-run (sudden stop / hazard)
        {
            let mut lead_motion = app.world_mut().get_mut::<CarMotion>(lead_moving).unwrap();
            lead_motion.stop_motion();
            let mut lead_car = app.world_mut().get_mut::<Car>(lead_moving).unwrap();
            lead_car.speed = 0.0;
        }

        // Step 100ms (2 steps): With driver reaction delay and only 6 units gap, crash is inevitable
        for _ in 0..2 {
            app.world_mut().resource_mut::<Time>().advance_by(dt);
            app.update();
        }

        // Verify crash detected!
        assert_eq!(
            app.world().resource::<CrashCounter>().0,
            1,
            "Rear-end crash must be detected after abrupt stop"
        );
        assert!(
            app.world().get::<Crashed>(lead_moving).is_some(),
            "Stopped lead car marked crashed"
        );
        assert!(
            app.world().get::<Crashed>(crash_follower).is_some(),
            "Follower marked crashed"
        );
        assert_eq!(
            app.world()
                .get::<CarMotion>(crash_follower)
                .unwrap()
                .current_speed,
            0.0
        );

        // Advance past 3.0s cleanup duration (3.1s = 62 steps)
        for _ in 0..62 {
            app.world_mut().resource_mut::<Time>().advance_by(dt);
            app.update();
        }

        // Verify wreck despawned
        assert!(
            app.world().get_entity(lead_moving).is_err(),
            "Wreck despawned after timer"
        );
        assert!(
            app.world().get_entity(crash_follower).is_err(),
            "Wreck despawned after timer"
        );
    }

    #[test]
    fn test_yellow_braking_never_commits_across_step_sizes_and_speeds() {
        let geom = RoadGeometry::default();
        let stop_pos = geom.stop_line_position(TrafficLane::Eastbound);
        let stop_line = lane_progress(TrafficLane::Eastbound, stop_pos, &geom);
        let east_y = geom.east_lane_y();

        for speed in [200.0, 240.0] {
            for dt_secs in [0.05, 0.10] {
                let mut app = App::new();
                app.init_resource::<Time>()
                    .init_resource::<Assets<Mesh>>()
                    .insert_resource(SpawnTimer::new(10000.0))
                    .insert_resource(geom.clone())
                    .insert_resource(TrafficSignal::new(TrafficLightState::Yellow, 1.5))
                    .add_systems(
                        Update,
                        (
                            tick_traffic_signal,
                            spawn_cars,
                            plan_car_motion,
                            apply_car_acceleration,
                            car_movement,
                        )
                            .chain(),
                    );

                // Position car with room to comfortably stop on yellow
                // Stop line mark is at 400.0 (x = -60.0). Place car at x = -300.0 (progress = 160.0, 240u before line)
                let car = app
                    .world_mut()
                    .spawn((
                        Car::new(speed, Vec2::X, CAR_LENGTH, CAR_WIDTH)
                            .with_lane(TrafficLane::Eastbound),
                        CarMotion::new(speed),
                        GroundPos(Vec2::new(-300.0, east_y)),
                        Heading::ZERO,
                    ))
                    .id();

                let dt = std::time::Duration::from_secs_f32(dt_secs);
                let total_steps = (4.0 / dt_secs) as usize; // 4 seconds covers 1.5s Yellow + 2.5s Red

                for step in 0..total_steps {
                    app.world_mut().resource_mut::<Time>().advance_by(dt);
                    app.update();

                    let car_comp = app.world().get::<Car>(car).unwrap();
                    let motion_comp = app.world().get::<CarMotion>(car).unwrap();
                    assert!(
                        !car_comp.committed_to_cross,
                        "Car must never commit to crossing when braking for yellow (speed={}, dt={}, step={})",
                        speed, dt_secs, step
                    );
                    assert!(
                        !motion_comp.committed_to_cross,
                        "Motion must never commit to crossing when braking for yellow (speed={}, dt={}, step={})",
                        speed, dt_secs, step
                    );
                }

                // Verify complete stop before stop line
                let final_motion = app.world().get::<CarMotion>(car).unwrap();
                let final_pos = app.world().get::<GroundPos>(car).unwrap().0;
                let final_car = app.world().get::<Car>(car).unwrap();
                let final_p = lane_progress(TrafficLane::Eastbound, final_pos, &geom);

                assert_eq!(
                    final_motion.current_speed, 0.0,
                    "Car must come to complete stop (speed={}, dt={}, final_spd={})",
                    speed, dt_secs, final_motion.current_speed
                );
                assert!(
                    final_car.front_progress(final_p) <= stop_line,
                    "Car must not cross stop line (speed={}, dt={}, front_p={}, stop_line={})",
                    speed,
                    dt_secs,
                    final_car.front_progress(final_p),
                    stop_line
                );
            }
        }
    }

    #[test]
    fn test_compute_signal_speed_limit_latched_stopping_and_clearing() {
        let stop_line_mark = 400.0;
        let stop_line_coord = 368.0;
        let decel = CAR_COMFORTABLE_DECELERATION; // 300.0

        // 1. Initial Yellow evaluation with ample distance:
        // Car is far away (center_p = 160.0, front_p = 184.0, dist_to_line = 216.0)
        // d_stop at 240.0 is 96.0 <= 216.0 -> stops and latches stopping_for_signal = true
        let (limit, committed, stopping) = compute_signal_speed_limit(
            TrafficLightState::Yellow,
            160.0,
            184.0,
            136.0,
            stop_line_mark,
            stop_line_coord,
            false,
            false,
            240.0,
            decel,
        );
        assert!(limit.is_some(), "Signal limit must be generated");
        assert!(!committed, "Car must not commit to cross");
        assert!(stopping, "Car must latch stopping_for_signal = true");

        // 2. Later Yellow evaluation with stopping_for_signal = true:
        // Car has moved closer such that unbuffered d_stop (96.0) > dist_to_line (80.0)
        // Latched stopping_for_signal prevents flipping to commit:
        let (limit2, committed2, stopping2) = compute_signal_speed_limit(
            TrafficLightState::Yellow,
            296.0,
            320.0,
            272.0,
            stop_line_mark,
            stop_line_coord,
            false,
            true,
            240.0,
            decel,
        );
        assert!(limit2.is_some(), "Signal limit must remain active");
        assert!(!committed2, "Latched stopping car must not commit to cross");
        assert!(stopping2, "stopping_for_signal must remain true");

        // 3. Dilemma zone case: car approaches on Yellow without prior stopping latch
        // dist_to_line = 50.0 < d_stop (96.0) -> must commit to crossing
        let (limit_dilemma, committed_dilemma, stopping_dilemma) = compute_signal_speed_limit(
            TrafficLightState::Yellow,
            326.0,
            350.0,
            302.0,
            stop_line_mark,
            stop_line_coord,
            false,
            false,
            240.0,
            decel,
        );
        assert!(
            limit_dilemma.is_none(),
            "Dilemma car has unrestricted limit"
        );
        assert!(committed_dilemma, "Dilemma car must commit to cross");
        assert!(
            !stopping_dilemma,
            "Dilemma car must not set stopping_for_signal"
        );

        // 4. Green signal clears stopping_for_signal:
        let (limit_green, committed_green, stopping_green) = compute_signal_speed_limit(
            TrafficLightState::Green,
            296.0,
            320.0,
            272.0,
            stop_line_mark,
            stop_line_coord,
            false,
            true,
            240.0,
            decel,
        );
        assert!(limit_green.is_none(), "Green has no restriction");
        assert!(!committed_green);
        assert!(!stopping_green, "Green must clear stopping_for_signal");

        // 5. Rear bumper clearing stop line clears both committed and stopping:
        let (limit_cleared, committed_cleared, stopping_cleared) = compute_signal_speed_limit(
            TrafficLightState::Yellow,
            450.0,
            474.0,
            426.0, // rear_progress (426.0) >= stop_line_mark (400.0)
            stop_line_mark,
            stop_line_coord,
            true,
            true,
            240.0,
            decel,
        );
        assert!(limit_cleared.is_none());
        assert!(
            !committed_cleared,
            "Rear cleared must reset committed_to_cross"
        );
        assert!(
            !stopping_cleared,
            "Rear cleared must reset stopping_for_signal"
        );
    }

    #[test]
    fn test_native_smoke_two_full_signal_cycles_integrated() {
        use crate::view3d::{
            Car3d, CarAssets3d, SignalAssets3d, SignalHead3d, SignalLens3d, View3dOf, on_add_car,
            setup_signals_3d, sync_3d_cars, sync_3d_signals,
        };

        let mut app = App::new();
        let geom = RoadGeometry::default();

        #[derive(Resource, Default)]
        struct SmokeMonitor {
            crashes_detected: usize,
            crashed_lead: Option<Entity>,
            crashed_follower: Option<Entity>,
            committed_car: Option<Entity>,
            committed_observed: bool,
            committed_cleared_stop_line: bool,
        }

        app.init_resource::<Time>()
            .init_resource::<SmokeMonitor>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<SignalAssets3d>()
            .init_resource::<CarAssets3d>()
            .insert_resource(geom.clone())
            .insert_resource(SpawnTimer::new(1.2))
            .insert_resource(TrafficSignal::default()) // Starts at Green, 6.0s
            .add_observer(
                |event: On<RearEndCrashEvent>, mut mon: ResMut<SmokeMonitor>| {
                    mon.crashes_detected += 1;
                    mon.crashed_lead = Some(event.leader);
                    mon.crashed_follower = Some(event.follower);
                },
            )
            .add_observer(on_add_car)
            .add_systems(Startup, setup_signals_3d)
            .add_systems(
                Update,
                (
                    (
                        tick_traffic_signal,
                        spawn_cars,
                        plan_car_motion,
                        apply_car_acceleration,
                        car_movement,
                        detect_rear_end_crashes,
                        tick_crash_cleanup,
                        car_cleanup,
                    )
                        .chain(),
                    sync_3d_signals.after(tick_traffic_signal),
                    sync_3d_cars.after(car_movement).after(car_cleanup),
                ),
            );

        app.update(); // run Startup to setup signal heads

        // Verify all 4 approach signal heads spawned with 3 lenses each
        let heads_count = app
            .world_mut()
            .query::<&SignalHead3d>()
            .iter(app.world())
            .count();
        assert_eq!(heads_count, 4, "Must spawn 4 approach signal heads");
        let lenses_count = app
            .world_mut()
            .query::<&SignalLens3d>()
            .iter(app.world())
            .count();
        assert_eq!(
            lenses_count, 12,
            "Must spawn 12 approach signal lenses (3 per head)"
        );

        let sb_stop_mark = lane_progress(
            TrafficLane::Southbound,
            geom.stop_line_position(TrafficLane::Southbound),
            &geom,
        );
        let mut dilemma_car = None;

        let dt = std::time::Duration::from_millis(50); // 50ms timestep (20 Hz)
        let total_steps = 600; // 30.0 seconds total (> 2 full cycles of 12.5s = 25.0s)
        let mut crash_injected = false;
        let mut injected_follower = None;
        let mut injected_leader = None;

        let car_assets = app.world().resource::<CarAssets3d>().clone();

        for step in 0..total_steps {
            let sim_time = step as f32 * 0.05;

            // Spawn dilemma-zone car on Southbound approach at t = 5.8s so it enters dilemma zone at t = 6.0s (Yellow transition)
            if dilemma_car.is_none() && sim_time >= 5.8 {
                let sb_lane = TrafficLane::Southbound;
                let sb_stop_pos = geom.stop_line_position(sb_lane);
                let spawn_pos = sb_stop_pos + Vec2::new(0.0, 60.0 + CAR_LENGTH / 2.0);
                let car_id = app
                    .world_mut()
                    .spawn((
                        Car::new(240.0, sb_lane.direction(), CAR_LENGTH, CAR_WIDTH)
                            .with_lane(sb_lane),
                        CarMotion::new(240.0),
                        GroundPos(spawn_pos),
                        sb_lane.spawn_heading(),
                    ))
                    .id();
                dilemma_car = Some(car_id);
                app.world_mut().resource_mut::<SmokeMonitor>().committed_car = Some(car_id);
            }

            // Inject an insufficient spacing / abrupt stop scenario at t = 14.0s (during Cycle 2 Green)
            if !crash_injected && sim_time >= 14.0 {
                crash_injected = true;
                let nb_lane = TrafficLane::Northbound;
                let nb_dir = nb_lane.direction();
                let lead = app
                    .world_mut()
                    .spawn((
                        Car::new(0.0, nb_dir, CAR_LENGTH, CAR_WIDTH).with_lane(nb_lane),
                        CarMotion::new(0.0),
                        GroundPos(Vec2::new(geom.north_lane_x(), -100.0)),
                        nb_lane.spawn_heading(),
                    ))
                    .id();
                // Spawn follower trailing closely at 200.0 with only 6.0 units bumper gap
                // Leader rear bumper = -100.0 - 24.0 = -124.0
                // Follower front bumper = -124.0 - 6.0 = -130.0
                // Follower center = -130.0 - 24.0 = -154.0
                let fol = app
                    .world_mut()
                    .spawn((
                        Car::new(200.0, nb_dir, CAR_LENGTH, CAR_WIDTH).with_lane(nb_lane),
                        CarMotion::new(200.0),
                        GroundPos(Vec2::new(geom.north_lane_x(), -154.0)),
                        nb_lane.spawn_heading(),
                    ))
                    .id();
                injected_leader = Some(lead);
                injected_follower = Some(fol);
            }

            app.world_mut().resource_mut::<Time>().advance_by(dt);
            app.update();

            // 1. Verify every approach obeys the same aspect across all signal heads
            let signal_state = app.world().resource::<TrafficSignal>().state;
            for (lens, vis) in app
                .world_mut()
                .query::<(&SignalLens3d, &Visibility)>()
                .iter(app.world())
            {
                let expected = if lens.state == signal_state {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                assert_eq!(
                    *vis, expected,
                    "At step {} (t={:.2}), lens for {:?} must be {:?} (current signal is {:?})",
                    step, sim_time, lens.state, expected, signal_state
                );
            }

            // 2. Verify queues do not overlap at normal spacing across all lanes
            for lane in TrafficLane::ALL {
                let mut lane_cars: Vec<(Entity, f32, f32, f32, bool)> = Vec::new();
                for (entity, car, ground_pos, crashed) in app
                    .world_mut()
                    .query::<(Entity, &Car, &GroundPos, Has<Crashed>)>()
                    .iter(app.world())
                {
                    if car.lane == Some(lane) {
                        let p = lane_progress(lane, ground_pos.0, &geom);
                        lane_cars.push((
                            entity,
                            p,
                            car.front_progress(p),
                            car.rear_progress(p),
                            crashed,
                        ));
                    }
                }
                lane_cars.sort_by(|a, b| b.1.total_cmp(&a.1));

                for i in 1..lane_cars.len() {
                    let leader = &lane_cars[i - 1];
                    let follower = &lane_cars[i];
                    // Skip only the deliberately injected crash test pair during their contact
                    if Some(leader.0) == injected_leader && Some(follower.0) == injected_follower {
                        continue;
                    }
                    let gap = bumper_gap(leader.3, follower.2);
                    assert!(
                        gap >= -0.1,
                        "At step {} (t={:.2}) in {:?}: follower {:?} overlapped leader {:?} with gap {}",
                        step,
                        sim_time,
                        lane,
                        follower.0,
                        leader.0,
                        gap
                    );
                }
            }

            // 3. Track and verify yellow-committed car clears the intersection
            let mut committed_now = false;
            let mut cleared_now = false;
            if let Some(car_entity) = app.world().resource::<SmokeMonitor>().committed_car
                && let Ok((car, motion, ground_pos)) = app
                    .world_mut()
                    .query::<(&Car, &CarMotion, &GroundPos)>()
                    .get(app.world(), car_entity)
            {
                if car.committed_to_cross || motion.committed_to_cross {
                    committed_now = true;
                    assert!(
                        motion.current_speed > 0.0,
                        "Yellow-committed car must not stop in intersection"
                    );
                }
                let p = lane_progress(TrafficLane::Southbound, ground_pos.0, &geom);
                if car.front_progress(p) >= sb_stop_mark {
                    cleared_now = true;
                }
            }
            if committed_now {
                app.world_mut()
                    .resource_mut::<SmokeMonitor>()
                    .committed_observed = true;
            }
            if cleared_now {
                app.world_mut()
                    .resource_mut::<SmokeMonitor>()
                    .committed_cleared_stop_line = true;
            }

            // 4. Verify crash state and 3D visual proxy for injected crash pair
            if let Some(fol_entity) = injected_follower
                && app.world().get::<Crashed>(fol_entity).is_some()
            {
                // Check that 3D proxy reflects crashed material
                let views: Vec<Entity> = app
                    .world_mut()
                    .query_filtered::<Entity, (With<Car3d>, With<View3dOf>)>()
                    .iter(app.world())
                    .filter(|e| {
                        app.world()
                            .get::<View3dOf>(*e)
                            .is_some_and(|v| v.0 == fol_entity)
                    })
                    .collect();
                if let Some(&view_ent) = views.first()
                    && let Some(mat_handle) = app
                        .world()
                        .get::<MeshMaterial3d<StandardMaterial>>(view_ent)
                {
                    assert_eq!(
                        mat_handle.0,
                        car_assets.crashed_material(),
                        "Crashed car 3D view must use crashed material"
                    );
                }
            }
        }

        // Final assertions:
        let monitor = app.world().resource::<SmokeMonitor>();
        assert!(
            monitor.crashes_detected >= 1,
            "Crash detection must remain functional at insufficient spacing"
        );
        assert!(
            monitor.committed_observed,
            "Yellow commitment must be observed for dilemma-zone car"
        );
        assert!(
            monitor.committed_cleared_stop_line,
            "Yellow-committed car must clear the stop line and intersection"
        );

        // 5. Verify wrecks cleared after crash cleanup duration
        if let Some(lead) = injected_leader {
            assert!(
                app.world().get_entity(lead).is_err(),
                "Injected crashed lead car must be despawned after cleanup"
            );
        }
        if let Some(fol) = injected_follower {
            assert!(
                app.world().get_entity(fol).is_err(),
                "Injected crashed follower car must be despawned after cleanup"
            );
        }

        // 6. Verify spawning recovered after crash cleanup and signal progression
        let nb_cars_end = app
            .world_mut()
            .query::<&Car>()
            .iter(app.world())
            .filter(|c| c.lane == Some(TrafficLane::Northbound))
            .count();
        assert!(
            nb_cars_end > 0,
            "Spawning in Northbound lane must recover after wreckage clears"
        );
    }
}
