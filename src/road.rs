use bevy::prelude::*;

use crate::constants::*;
use crate::traffic::TrafficLane;

/// Color constants for roads and grid
pub const COLOR_ROAD_ASPHALT: Color = Color::srgb(0.18, 0.18, 0.20);
pub const COLOR_ROAD_BORDER: Color = Color::srgb(0.85, 0.85, 0.88);
pub const COLOR_ROAD_CENTERLINE: Color = Color::srgb(0.95, 0.80, 0.10);
pub const COLOR_GRID_LINE: Color = Color::srgba(0.20, 0.22, 0.28, 0.40);
pub const COLOR_ROAD_STOP_LINE: Color = Color::srgb(0.92, 0.92, 0.92);

pub const ROAD_BORDER_THICKNESS: f32 = 3.0;
pub const ROAD_CENTERLINE_THICKNESS: f32 = 3.0;
pub const STOP_LINE_THICKNESS: f32 = 6.0;
/// Classification of a road visual piece
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoadPieceKind {
    Asphalt,
    Centerline,
    Border,
    StopLine,
}

/// A flat rectangular piece of road surface or marking
#[derive(Debug, Clone, PartialEq)]
pub struct RoadPiece {
    pub center: Vec2,
    pub size: Vec2,
    pub angle: f32,
    pub kind: RoadPieceKind,
}

/// Orientation of a road
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadOrientation {
    Horizontal,
    Vertical,
    Curved,
}

/// Tag component for a road entity
#[derive(Component, Debug, Clone)]
pub struct Road {
    pub orientation: RoadOrientation,
    pub width: f32,
    pub length: f32,
}

/// Tag component for static visual markings of roads (borders, centerlines)
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadMarkingType {
    Border,
    Centerline,
}

#[derive(Component, Debug, Clone)]
pub struct RoadMarking {
    pub marking_type: RoadMarkingType,
}

/// Tag component for background grid line sprites
#[derive(Component, Debug, Clone)]
pub struct GridLine;

/// Lane travel direction
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneDirection {
    /// East (+X) for horizontal, North (+Y) for vertical
    Positive,
    /// West (-X) for horizontal, South (-Y) for vertical
    Negative,
}

/// Lane specification for traffic routing
#[derive(Component, Debug, Clone)]
pub struct Lane {
    pub orientation: RoadOrientation,
    pub direction: LaneDirection,
    pub center_offset: f32,
    pub width: f32,
}

/// Road geometry resource providing geometric queries for the intersection
#[derive(Resource, Debug, Clone)]
pub struct RoadGeometry {
    pub road_width: f32,
    pub lane_width: f32,
    pub world_width: f32,
    pub world_height: f32,
    pub intersection_bounds: Rect,
    pub curve_start_x: f32,
    pub curve_center: Vec2,
    pub curve_radius: f32,
}

impl Default for RoadGeometry {
    fn default() -> Self {
        let half_road = ROAD_WIDTH / 2.0;
        Self {
            road_width: ROAD_WIDTH,
            lane_width: LANE_WIDTH,
            world_width: WORLD_WIDTH,
            world_height: WORLD_HEIGHT,
            intersection_bounds: Rect::from_corners(
                Vec2::new(-half_road, -half_road),
                Vec2::new(half_road, half_road),
            ),
            curve_start_x: CURVE_START_X,
            curve_center: CURVE_CENTER,
            curve_radius: CURVE_RADIUS,
        }
    }
}

impl RoadGeometry {
    pub fn new(road_width: f32, world_width: f32, world_height: f32) -> Self {
        let half_road = road_width / 2.0;
        Self {
            road_width,
            lane_width: road_width / 2.0,
            world_width,
            world_height,
            intersection_bounds: Rect::from_corners(
                Vec2::new(-half_road, -half_road),
                Vec2::new(half_road, half_road),
            ),
            curve_start_x: CURVE_START_X,
            curve_center: CURVE_CENTER,
            curve_radius: CURVE_RADIUS,
        }
    }

    /// Check if a 2D coordinate is inside the central intersection square
    pub fn is_in_intersection(&self, point: Vec2) -> bool {
        self.intersection_bounds.contains(point)
    }

    /// Bounds of the straight horizontal road strip (west of the curve)
    pub fn horizontal_bounds(&self) -> Rect {
        let half_road = self.road_width / 2.0;
        let half_w = self.world_width / 2.0;
        Rect::from_corners(
            Vec2::new(-half_w, -half_road),
            Vec2::new(self.curve_start_x, half_road),
        )
    }

    /// Bounds of the vertical road strip
    pub fn vertical_bounds(&self) -> Rect {
        let half_road = self.road_width / 2.0;
        let half_h = self.world_height / 2.0;
        Rect::from_corners(Vec2::new(-half_road, -half_h), Vec2::new(half_road, half_h))
    }

    /// Check if a 2D coordinate is on the curved road section
    pub fn is_on_curved_road(&self, point: Vec2) -> bool {
        let half_road = self.road_width / 2.0;
        if point.x < self.curve_start_x - 1.0 || point.y > self.curve_center.y + 1.0 {
            return false;
        }
        let dist = point.distance(self.curve_center);
        dist >= (self.curve_radius - half_road) && dist <= (self.curve_radius + half_road)
    }

    /// Check if a 2D coordinate is anywhere on the paved road network
    pub fn is_on_road(&self, point: Vec2) -> bool {
        self.horizontal_bounds().contains(point)
            || self.vertical_bounds().contains(point)
            || self.is_on_curved_road(point)
    }

    /// Center position for horizontal Eastbound lane (lower lane, y < 0)
    pub fn east_lane_y(&self) -> f32 {
        -self.lane_width / 2.0
    }

    /// Center position for horizontal Westbound lane (upper lane, y > 0)
    pub fn west_lane_y(&self) -> f32 {
        self.lane_width / 2.0
    }

    /// Center position for vertical Southbound lane (left lane, x < 0)
    pub fn south_lane_x(&self) -> f32 {
        -self.lane_width / 2.0
    }

    /// Center position for vertical Northbound lane (right lane, x > 0)
    pub fn north_lane_x(&self) -> f32 {
        self.lane_width / 2.0
    }

    /// Radius of the Eastbound outer lane on the curve
    pub fn curve_east_lane_radius(&self) -> f32 {
        self.curve_radius + self.lane_width / 2.0
    }

    /// Radius of the Westbound inner lane on the curve
    pub fn curve_west_lane_radius(&self) -> f32 {
        self.curve_radius - self.lane_width / 2.0
    }

    /// X coordinate where the curved Westbound lane enters/exits at the northern edge
    pub fn curve_west_lane_x(&self) -> f32 {
        self.curve_center.x + self.curve_west_lane_radius()
    }

    /// X coordinate where the curved Eastbound lane enters/exits at the northern edge
    pub fn curve_east_lane_x(&self) -> f32 {
        self.curve_center.x + self.curve_east_lane_radius()
    }

    /// Stop line position (center of the stop line marking) for a given lane approach,
    /// derived directly from `intersection_bounds` and lane coordinates.
    pub fn stop_line_position(&self, lane: TrafficLane) -> Vec2 {
        match lane {
            TrafficLane::Eastbound => Vec2::new(self.intersection_bounds.min.x, self.east_lane_y()),
            TrafficLane::Westbound => Vec2::new(self.intersection_bounds.max.x, self.west_lane_y()),
            TrafficLane::Southbound => {
                Vec2::new(self.south_lane_x(), self.intersection_bounds.max.y)
            }
            TrafficLane::Northbound => {
                Vec2::new(self.north_lane_x(), self.intersection_bounds.min.y)
            }
        }
    }

    /// Stop line dimensions (thickness along travel direction, width across the lane)
    /// derived directly from `lane_width` and `STOP_LINE_THICKNESS`.
    pub fn stop_line_size(&self, lane: TrafficLane) -> Vec2 {
        match lane {
            TrafficLane::Eastbound | TrafficLane::Westbound => {
                Vec2::new(STOP_LINE_THICKNESS, self.lane_width)
            }
            TrafficLane::Southbound | TrafficLane::Northbound => {
                Vec2::new(self.lane_width, STOP_LINE_THICKNESS)
            }
        }
    }

    /// Stop line bounding box derived directly from `stop_line_position` and `stop_line_size`.
    pub fn stop_line_bounds(&self, lane: TrafficLane) -> Rect {
        Rect::from_center_size(self.stop_line_position(lane), self.stop_line_size(lane))
    }

    /// Progress coordinate where a car center comes to rest before an intersection stop line,
    /// stopped one half-car-length plus a small buffer before `intersection_bounds`.
    pub fn stop_line_progress(&self, lane: TrafficLane) -> f32 {
        crate::traffic::stop_line_progress(lane, self)
    }

    /// Derived signal pole / head position for a given approach.
    /// Placed at the roadside curb on the driver's right-hand approach side of the stop line,
    /// dynamically derived from `intersection_bounds` without hardcoded world coordinates.
    pub fn signal_position(&self, lane: TrafficLane) -> Vec2 {
        let curb_offset = 8.0;
        let stop_pos = self.stop_line_position(lane);
        match lane {
            TrafficLane::Eastbound => {
                // Driver travelling +X; right-hand roadside is south (-Y)
                Vec2::new(stop_pos.x, self.intersection_bounds.min.y - curb_offset)
            }
            TrafficLane::Westbound => {
                // Driver travelling -X; right-hand roadside is north (+Y)
                Vec2::new(stop_pos.x, self.intersection_bounds.max.y + curb_offset)
            }
            TrafficLane::Southbound => {
                // Driver travelling -Y; right-hand roadside is west (-X)
                Vec2::new(self.intersection_bounds.min.x - curb_offset, stop_pos.y)
            }
            TrafficLane::Northbound => {
                // Driver travelling +Y; right-hand roadside is east (+X)
                Vec2::new(self.intersection_bounds.max.x + curb_offset, stop_pos.y)
            }
        }
    }

    /// Heading angle (in radians) that an approach-facing signal head should point towards oncoming traffic.
    pub fn signal_facing_heading(&self, lane: TrafficLane) -> f32 {
        match lane {
            TrafficLane::Eastbound => std::f32::consts::PI,
            TrafficLane::Westbound => 0.0,
            TrafficLane::Southbound => std::f32::consts::FRAC_PI_2,
            TrafficLane::Northbound => -std::f32::consts::FRAC_PI_2,
        }
    }
}

/// Generate all visual road pieces (asphalt and markings) from road geometry
pub fn road_pieces(geom: &RoadGeometry) -> Vec<RoadPiece> {
    let mut pieces = Vec::with_capacity(138);

    let half_w = geom.world_width / 2.0;
    let half_h = geom.world_height / 2.0;
    let half_road = geom.road_width / 2.0;
    let border_thickness = ROAD_BORDER_THICKNESS;
    let centerline_thickness = ROAD_CENTERLINE_THICKNESS;

    // Straight Horizontal Road Asphalt (from -half_w to geom.curve_start_x)
    let h_straight_len = half_w + geom.curve_start_x;
    let h_straight_center_x = (-half_w + geom.curve_start_x) / 2.0;
    pieces.push(RoadPiece {
        center: Vec2::new(h_straight_center_x, 0.0),
        size: Vec2::new(h_straight_len, geom.road_width),
        angle: 0.0,
        kind: RoadPieceKind::Asphalt,
    });

    // Vertical Road Asphalt
    pieces.push(RoadPiece {
        center: Vec2::new(0.0, 0.0),
        size: Vec2::new(geom.road_width, geom.world_height),
        angle: 0.0,
        kind: RoadPieceKind::Asphalt,
    });

    // Horizontal Straight Markings
    // Top border (y = half_road, from -half_w to geom.curve_start_x)
    pieces.push(RoadPiece {
        center: Vec2::new(h_straight_center_x, half_road),
        size: Vec2::new(h_straight_len, border_thickness),
        angle: 0.0,
        kind: RoadPieceKind::Border,
    });

    // Bottom border (y = -half_road, from -half_w to geom.curve_start_x)
    pieces.push(RoadPiece {
        center: Vec2::new(h_straight_center_x, -half_road),
        size: Vec2::new(h_straight_len, border_thickness),
        angle: 0.0,
        kind: RoadPieceKind::Border,
    });

    // West Centerline segment (from -half_w to -half_road)
    let west_seg_len = half_w - half_road;
    let west_center_x = -half_road - (west_seg_len / 2.0);
    pieces.push(RoadPiece {
        center: Vec2::new(west_center_x, 0.0),
        size: Vec2::new(west_seg_len, centerline_thickness),
        angle: 0.0,
        kind: RoadPieceKind::Centerline,
    });

    // East Straight Centerline segment (from half_road to geom.curve_start_x)
    let east_straight_seg_len = geom.curve_start_x - half_road;
    let east_straight_center_x = half_road + (east_straight_seg_len / 2.0);
    pieces.push(RoadPiece {
        center: Vec2::new(east_straight_center_x, 0.0),
        size: Vec2::new(east_straight_seg_len, centerline_thickness),
        angle: 0.0,
        kind: RoadPieceKind::Centerline,
    });

    // Vertical Road Markings
    // Left border (x = -half_road)
    pieces.push(RoadPiece {
        center: Vec2::new(-half_road, 0.0),
        size: Vec2::new(border_thickness, geom.world_height),
        angle: 0.0,
        kind: RoadPieceKind::Border,
    });

    // Right border (x = half_road)
    pieces.push(RoadPiece {
        center: Vec2::new(half_road, 0.0),
        size: Vec2::new(border_thickness, geom.world_height),
        angle: 0.0,
        kind: RoadPieceKind::Border,
    });

    // South Centerline segment (from -half_h to -half_road)
    let south_seg_len = half_h - half_road;
    let south_center_y = -half_road - (south_seg_len / 2.0);
    pieces.push(RoadPiece {
        center: Vec2::new(0.0, south_center_y),
        size: Vec2::new(centerline_thickness, south_seg_len),
        angle: 0.0,
        kind: RoadPieceKind::Centerline,
    });

    // North Centerline segment (from half_road to half_h)
    let north_seg_len = half_h - half_road;
    let north_center_y = half_road + (north_seg_len / 2.0);
    pieces.push(RoadPiece {
        center: Vec2::new(0.0, north_center_y),
        size: Vec2::new(centerline_thickness, north_seg_len),
        angle: 0.0,
        kind: RoadPieceKind::Centerline,
    });

    // Curved Road Markings and Asphalt Segments
    let num_curve_steps = 32;
    let d_theta = std::f32::consts::FRAC_PI_2 / (num_curve_steps as f32);
    let asphalt_seg_len = geom.curve_radius * d_theta * 1.10;
    let centerline_seg_len = geom.curve_radius * d_theta * 1.05;
    let inner_border_radius = geom.curve_radius - half_road;
    let outer_border_radius = geom.curve_radius + half_road;
    let inner_border_seg_len = inner_border_radius * d_theta * 1.05;
    let outer_border_seg_len = outer_border_radius * d_theta * 1.05;

    for i in 0..num_curve_steps {
        let theta = (i as f32 + 0.5) * d_theta;

        let pos_asphalt = Vec2::new(
            geom.curve_center.x + geom.curve_radius * theta.sin(),
            geom.curve_center.y - geom.curve_radius * theta.cos(),
        );
        let pos_inner = Vec2::new(
            geom.curve_center.x + inner_border_radius * theta.sin(),
            geom.curve_center.y - inner_border_radius * theta.cos(),
        );
        let pos_outer = Vec2::new(
            geom.curve_center.x + outer_border_radius * theta.sin(),
            geom.curve_center.y - outer_border_radius * theta.cos(),
        );

        // Asphalt segment
        pieces.push(RoadPiece {
            center: pos_asphalt,
            size: Vec2::new(asphalt_seg_len, geom.road_width),
            angle: theta,
            kind: RoadPieceKind::Asphalt,
        });

        // Centerline segment
        pieces.push(RoadPiece {
            center: pos_asphalt,
            size: Vec2::new(centerline_seg_len, centerline_thickness),
            angle: theta,
            kind: RoadPieceKind::Centerline,
        });

        // Inner border segment
        pieces.push(RoadPiece {
            center: pos_inner,
            size: Vec2::new(inner_border_seg_len, border_thickness),
            angle: theta,
            kind: RoadPieceKind::Border,
        });

        // Outer border segment
        pieces.push(RoadPiece {
            center: pos_outer,
            size: Vec2::new(outer_border_seg_len, border_thickness),
            angle: theta,
            kind: RoadPieceKind::Border,
        });
    }

    // Stop Line Markings (one per approach)
    for lane in TrafficLane::ALL {
        pieces.push(RoadPiece {
            center: geom.stop_line_position(lane),
            size: geom.stop_line_size(lane),
            angle: 0.0,
            kind: RoadPieceKind::StopLine,
        });
    }

    pieces
}

/// Bevy plugin configuring road geometry and static road/grid entity spawning
pub struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadGeometry>()
            .add_systems(Startup, setup_roads);
    }
}

/// Spawn static road and lane data entities (without visual sprites)
pub fn setup_roads(mut commands: Commands, road_geom: Res<RoadGeometry>) {
    let half_w = road_geom.world_width / 2.0;
    let h_straight_len = half_w + road_geom.curve_start_x;
    let h_straight_center_x = (-half_w + road_geom.curve_start_x) / 2.0;
    let curve_arc_len = std::f32::consts::FRAC_PI_2 * road_geom.curve_radius;

    // Straight Horizontal Road (from -half_w to CURVE_START_X)
    commands
        .spawn((
            Transform::from_xyz(h_straight_center_x, 0.0, 0.0),
            Road {
                orientation: RoadOrientation::Horizontal,
                width: road_geom.road_width,
                length: h_straight_len,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Lane {
                    orientation: RoadOrientation::Horizontal,
                    direction: LaneDirection::Positive,
                    center_offset: road_geom.east_lane_y(),
                    width: road_geom.lane_width,
                },
                Transform::from_xyz(0.0, road_geom.east_lane_y(), 0.0),
            ));
            parent.spawn((
                Lane {
                    orientation: RoadOrientation::Horizontal,
                    direction: LaneDirection::Negative,
                    center_offset: road_geom.west_lane_y(),
                    width: road_geom.lane_width,
                },
                Transform::from_xyz(0.0, road_geom.west_lane_y(), 0.0),
            ));
        });

    // Vertical Road
    commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            Road {
                orientation: RoadOrientation::Vertical,
                width: road_geom.road_width,
                length: road_geom.world_height,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Lane {
                    orientation: RoadOrientation::Vertical,
                    direction: LaneDirection::Positive,
                    center_offset: road_geom.north_lane_x(),
                    width: road_geom.lane_width,
                },
                Transform::from_xyz(road_geom.north_lane_x(), 0.0, 0.0),
            ));
            parent.spawn((
                Lane {
                    orientation: RoadOrientation::Vertical,
                    direction: LaneDirection::Negative,
                    center_offset: road_geom.south_lane_x(),
                    width: road_geom.lane_width,
                },
                Transform::from_xyz(road_geom.south_lane_x(), 0.0, 0.0),
            ));
        });

    // Curved Road: quarter-circle arc from (CURVE_START_X, 0.0) to (CURVE_START_X + CURVE_RADIUS, CURVE_CENTER.y)
    commands
        .spawn((
            Transform::from_xyz(road_geom.curve_start_x, road_geom.curve_center.y, 0.0),
            Road {
                orientation: RoadOrientation::Curved,
                width: road_geom.road_width,
                length: curve_arc_len,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Lane {
                    orientation: RoadOrientation::Curved,
                    direction: LaneDirection::Positive,
                    center_offset: road_geom.lane_width / 2.0,
                    width: road_geom.lane_width,
                },
                Transform::default(),
            ));
            parent.spawn((
                Lane {
                    orientation: RoadOrientation::Curved,
                    direction: LaneDirection::Negative,
                    center_offset: -road_geom.lane_width / 2.0,
                    width: road_geom.lane_width,
                },
                Transform::default(),
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_road_geometry_bounds() {
        let geom = RoadGeometry::default();

        // Intersection bounds check
        assert!(geom.is_in_intersection(Vec2::ZERO));
        assert!(geom.is_in_intersection(Vec2::new(50.0, 50.0)));
        assert!(geom.is_in_intersection(Vec2::new(-50.0, -50.0)));
        assert!(!geom.is_in_intersection(Vec2::new(70.0, 70.0)));
        assert!(!geom.is_in_intersection(Vec2::new(0.0, 100.0)));

        // On-road check
        assert!(geom.is_on_road(Vec2::ZERO));
        assert!(geom.is_on_road(Vec2::new(200.0, 0.0))); // horizontal road
        assert!(geom.is_on_road(Vec2::new(0.0, -300.0))); // vertical road
        assert!(geom.is_on_road(Vec2::new(50.0, 300.0))); // within vertical road width
        assert!(!geom.is_on_road(Vec2::new(200.0, 200.0))); // corner off-road
    }

    #[test]
    fn test_lane_center_coordinates() {
        let geom = RoadGeometry::default();

        assert_eq!(geom.east_lane_y(), -30.0);
        assert_eq!(geom.west_lane_y(), 30.0);
        assert_eq!(geom.south_lane_x(), -30.0);
        assert_eq!(geom.north_lane_x(), 30.0);

        // All lane centers should be within the road bounds
        assert!(geom.is_on_road(Vec2::new(0.0, geom.east_lane_y())));
        assert!(geom.is_on_road(Vec2::new(0.0, geom.west_lane_y())));
        assert!(geom.is_on_road(Vec2::new(geom.south_lane_x(), 0.0)));
        assert!(geom.is_on_road(Vec2::new(geom.north_lane_x(), 0.0)));
    }

    #[test]
    fn test_setup_roads_spawns_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<RoadGeometry>()
            .add_systems(Startup, setup_roads);
        app.update();

        // Check road entities
        let mut road_query = app.world_mut().query::<(&Road, &Transform)>();
        let roads: Vec<(&Road, &Transform)> = road_query.iter(app.world()).collect();
        assert_eq!(
            roads.len(),
            3,
            "Should spawn horizontal, vertical, and curved roads"
        );

        for (road, transform) in &roads {
            assert_eq!(transform.translation.z, 0.0);
            assert_eq!(road.width, ROAD_WIDTH);
        }

        // Check lane entities spawned under roads
        let mut lane_query = app.world_mut().query::<&Lane>();
        let lanes: Vec<&Lane> = lane_query.iter(app.world()).collect();
        assert_eq!(
            lanes.len(),
            6,
            "Should spawn 2 lanes for each of the 3 roads"
        );
    }

    #[test]
    fn test_road_and_marking_exact_coordinates() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<RoadGeometry>()
            .add_systems(Startup, setup_roads);
        app.update();

        // Verify horizontal road at y = 0, vertical road at x = 0, curved road at arc origin
        let mut road_query = app.world_mut().query::<(&Road, &Transform)>();
        let mut found_h = false;
        let mut found_v = false;
        let mut found_c = false;
        for (road, transform) in road_query.iter(app.world()) {
            match road.orientation {
                RoadOrientation::Horizontal => {
                    assert_eq!(transform.translation.z, 0.0);
                    assert_eq!(transform.translation.y, 0.0);
                    found_h = true;
                }
                RoadOrientation::Vertical => {
                    assert_eq!(transform.translation, Vec3::new(0.0, 0.0, 0.0));
                    found_v = true;
                }
                RoadOrientation::Curved => {
                    assert_eq!(
                        transform.translation,
                        Vec3::new(CURVE_START_X, CURVE_CENTER.y, 0.0)
                    );
                    found_c = true;
                }
            }
        }
        assert!(found_h && found_v && found_c);
    }
    #[test]
    fn test_curved_road_geometry_queries() {
        let geom = RoadGeometry::default();

        // Start of curve (at x = 160, y = 0)
        assert!(geom.is_on_curved_road(Vec2::new(160.0, 0.0)));
        assert!(geom.is_on_road(Vec2::new(160.0, 0.0)));

        // Midpoint of curve around 45 degrees
        let angle = std::f32::consts::FRAC_PI_4;
        let mid_point = Vec2::new(
            geom.curve_center.x + geom.curve_radius * angle.sin(),
            geom.curve_center.y - geom.curve_radius * angle.cos(),
        );
        assert!(geom.is_on_curved_road(mid_point));
        assert!(geom.is_on_road(mid_point));

        // Outer and inner edges of the curve
        let outer_point = Vec2::new(
            geom.curve_center.x + (geom.curve_radius + 50.0) * angle.sin(),
            geom.curve_center.y - (geom.curve_radius + 50.0) * angle.cos(),
        );
        assert!(geom.is_on_curved_road(outer_point));

        // Off the curve (too far inside or outside)
        let far_inside = Vec2::new(
            geom.curve_center.x + (geom.curve_radius - 100.0) * angle.sin(),
            geom.curve_center.y - (geom.curve_radius - 100.0) * angle.cos(),
        );
        assert!(!geom.is_on_curved_road(far_inside));

        // End of curve at the northern boundary (x = 660, y = 500)
        let end_point = Vec2::new(geom.curve_center.x + geom.curve_radius, geom.curve_center.y);
        assert!(geom.is_on_curved_road(end_point));
        assert!(geom.is_on_road(end_point));
    }

    #[test]
    fn test_road_pieces_counts_and_properties() {
        let geom = RoadGeometry::default();
        let pieces = road_pieces(&geom);

        let asphalt_pieces: Vec<&RoadPiece> = pieces
            .iter()
            .filter(|p| p.kind == RoadPieceKind::Asphalt)
            .collect();
        let centerline_pieces: Vec<&RoadPiece> = pieces
            .iter()
            .filter(|p| p.kind == RoadPieceKind::Centerline)
            .collect();
        let border_pieces: Vec<&RoadPiece> = pieces
            .iter()
            .filter(|p| p.kind == RoadPieceKind::Border)
            .collect();
        let stop_line_pieces: Vec<&RoadPiece> = pieces
            .iter()
            .filter(|p| p.kind == RoadPieceKind::StopLine)
            .collect();

        // 1 horizontal + 1 vertical + 32 curved = 34 asphalt pieces
        assert_eq!(asphalt_pieces.len(), 34);
        // 2 horizontal + 2 vertical + 32 curved = 36 centerline pieces
        assert_eq!(centerline_pieces.len(), 36);
        // 2 horizontal + 2 vertical + 64 curved (inner + outer) = 68 border pieces
        assert_eq!(border_pieces.len(), 68);
        // 4 approach stop lines
        assert_eq!(stop_line_pieces.len(), 4);
        assert_eq!(pieces.len(), 142);

        // Check stop lines match geom.stop_line_position and size for each approach
        for lane in TrafficLane::ALL {
            let p = stop_line_pieces
                .iter()
                .find(|p| p.center == geom.stop_line_position(lane))
                .unwrap_or_else(|| panic!("stop line for {:?} must exist", lane));
            assert_eq!(p.size, geom.stop_line_size(lane));
        }

        // Check straight horizontal asphalt piece
        let h_straight = asphalt_pieces
            .iter()
            .find(|p| p.center.y == 0.0 && p.angle == 0.0 && p.size.y == ROAD_WIDTH)
            .expect("should find straight horizontal asphalt");
        assert_eq!(h_straight.size.x, HALF_WORLD_WIDTH + CURVE_START_X);

        // Check vertical asphalt piece
        let v_straight = asphalt_pieces
            .iter()
            .find(|p| p.center == Vec2::ZERO && p.angle == 0.0 && p.size.x == ROAD_WIDTH)
            .expect("should find vertical asphalt");
        assert_eq!(v_straight.size.y, WORLD_HEIGHT);

        // Check that all pieces have non-zero dimensions
        for piece in &pieces {
            assert!(piece.size.x > 0.0);
            assert!(piece.size.y > 0.0);
        }
    }

    #[test]
    fn test_stop_line_and_signal_placement_derived_from_road_geometry() {
        let geom = RoadGeometry::default();

        // 1. Verify default geometry stop line placements
        let half_road = ROAD_WIDTH / 2.0;
        let half_lane = LANE_WIDTH / 2.0;

        let stop_eb = geom.stop_line_position(TrafficLane::Eastbound);
        assert_eq!(stop_eb, Vec2::new(-half_road, -half_lane));
        assert_eq!(
            geom.stop_line_size(TrafficLane::Eastbound),
            Vec2::new(STOP_LINE_THICKNESS, LANE_WIDTH)
        );
        let bounds_eb = geom.stop_line_bounds(TrafficLane::Eastbound);
        assert_eq!(bounds_eb.center(), stop_eb);

        let stop_wb = geom.stop_line_position(TrafficLane::Westbound);
        assert_eq!(stop_wb, Vec2::new(half_road, half_lane));
        assert_eq!(
            geom.stop_line_size(TrafficLane::Westbound),
            Vec2::new(STOP_LINE_THICKNESS, LANE_WIDTH)
        );

        let stop_sb = geom.stop_line_position(TrafficLane::Southbound);
        assert_eq!(stop_sb, Vec2::new(-half_lane, half_road));
        assert_eq!(
            geom.stop_line_size(TrafficLane::Southbound),
            Vec2::new(LANE_WIDTH, STOP_LINE_THICKNESS)
        );

        let stop_nb = geom.stop_line_position(TrafficLane::Northbound);
        assert_eq!(stop_nb, Vec2::new(half_lane, -half_road));
        assert_eq!(
            geom.stop_line_size(TrafficLane::Northbound),
            Vec2::new(LANE_WIDTH, STOP_LINE_THICKNESS)
        );

        // 2. Verify signal placements and headings
        let sig_eb = geom.signal_position(TrafficLane::Eastbound);
        assert_eq!(sig_eb, Vec2::new(-half_road, -half_road - 8.0));
        assert_eq!(
            geom.signal_facing_heading(TrafficLane::Eastbound),
            std::f32::consts::PI
        );

        let sig_wb = geom.signal_position(TrafficLane::Westbound);
        assert_eq!(sig_wb, Vec2::new(half_road, half_road + 8.0));
        assert_eq!(geom.signal_facing_heading(TrafficLane::Westbound), 0.0);

        let sig_sb = geom.signal_position(TrafficLane::Southbound);
        assert_eq!(sig_sb, Vec2::new(-half_road - 8.0, half_road));
        assert_eq!(
            geom.signal_facing_heading(TrafficLane::Southbound),
            std::f32::consts::FRAC_PI_2
        );

        let sig_nb = geom.signal_position(TrafficLane::Northbound);
        assert_eq!(sig_nb, Vec2::new(half_road + 8.0, -half_road));
        assert_eq!(
            geom.signal_facing_heading(TrafficLane::Northbound),
            -std::f32::consts::FRAC_PI_2
        );

        // 3. Verify parameterized road geometry scales positions dynamically (no hardcoded world constants)
        let custom_road_w = 320.0;
        let custom_geom = RoadGeometry::new(custom_road_w, 2000.0, 1500.0);
        let custom_half_road = 160.0;
        let custom_half_lane = 80.0;

        assert_eq!(
            custom_geom.stop_line_position(TrafficLane::Eastbound),
            Vec2::new(-custom_half_road, -custom_half_lane)
        );
        assert_eq!(
            custom_geom.stop_line_size(TrafficLane::Eastbound),
            Vec2::new(STOP_LINE_THICKNESS, custom_road_w / 2.0)
        );
        assert_eq!(
            custom_geom.signal_position(TrafficLane::Eastbound),
            Vec2::new(-custom_half_road, -custom_half_road - 8.0)
        );
        assert_eq!(
            custom_geom.signal_position(TrafficLane::Westbound),
            Vec2::new(custom_half_road, custom_half_road + 8.0)
        );
    }
}
