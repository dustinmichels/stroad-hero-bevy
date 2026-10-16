use bevy::prelude::*;

use crate::constants::*;

/// Color constants for roads and grid
pub const COLOR_ROAD_ASPHALT: Color = Color::srgb(0.18, 0.18, 0.20);
pub const COLOR_ROAD_BORDER: Color = Color::srgb(0.85, 0.85, 0.88);
pub const COLOR_ROAD_CENTERLINE: Color = Color::srgb(0.95, 0.80, 0.10);
pub const COLOR_GRID_LINE: Color = Color::srgba(0.20, 0.22, 0.28, 0.40);

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
}

/// Bevy plugin configuring road geometry and static road/grid entity spawning
pub struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadGeometry>()
            .add_systems(Startup, setup_roads);
    }
}

/// Spawn static road asphalt rectangles and line markings at Z_ROAD layer (beneath cars and player)
pub fn setup_roads(mut commands: Commands) {
    let half_w = HALF_WORLD_WIDTH;
    let half_h = HALF_WORLD_HEIGHT;
    let half_road = ROAD_WIDTH / 2.0;
    let border_thickness = 3.0;
    let centerline_thickness = 3.0;
    let grid_line_thickness = 1.0;

    // Background grid lines (z = Z_ROAD - 0.01, faint lines behind asphalt)
    let z_grid = Z_ROAD - 0.01;
    let mut x = -half_w;
    while x <= half_w {
        commands.spawn((
            Sprite::from_color(
                COLOR_GRID_LINE,
                Vec2::new(grid_line_thickness, WORLD_HEIGHT),
            ),
            Transform::from_xyz(x, 0.0, z_grid),
            GridLine,
        ));
        x += GRID_STEP_SIZE;
    }

    let mut y = -half_h;
    while y <= half_h {
        commands.spawn((
            Sprite::from_color(COLOR_GRID_LINE, Vec2::new(WORLD_WIDTH, grid_line_thickness)),
            Transform::from_xyz(0.0, y, z_grid),
            GridLine,
        ));
        y += GRID_STEP_SIZE;
    }

    // Straight Horizontal Road Asphalt (from -half_w to CURVE_START_X)
    let h_straight_len = half_w + CURVE_START_X;
    let h_straight_center_x = (-half_w + CURVE_START_X) / 2.0;
    commands.spawn((
        Sprite::from_color(COLOR_ROAD_ASPHALT, Vec2::new(h_straight_len, ROAD_WIDTH)),
        Transform::from_xyz(h_straight_center_x, 0.0, Z_ROAD),
        Road {
            orientation: RoadOrientation::Horizontal,
            width: ROAD_WIDTH,
            length: h_straight_len,
        },
    ));

    // Vertical Road Asphalt (z = Z_ROAD)
    commands.spawn((
        Sprite::from_color(COLOR_ROAD_ASPHALT, Vec2::new(ROAD_WIDTH, WORLD_HEIGHT)),
        Transform::from_xyz(0.0, 0.0, Z_ROAD),
        Road {
            orientation: RoadOrientation::Vertical,
            width: ROAD_WIDTH,
            length: WORLD_HEIGHT,
        },
    ));

    // Curved Road: quarter-circle arc from (CURVE_START_X, 0.0) to (CURVE_START_X + CURVE_RADIUS, CURVE_CENTER.y)
    let curve_arc_len = std::f32::consts::FRAC_PI_2 * CURVE_RADIUS;
    commands.spawn((
        Transform::from_xyz(CURVE_START_X, CURVE_CENTER.y, Z_ROAD),
        Road {
            orientation: RoadOrientation::Curved,
            width: ROAD_WIDTH,
            length: curve_arc_len,
        },
    ));

    let z_marking = Z_ROAD + 0.01;

    // --- Horizontal Straight Markings ---
    // Top border (y = half_road, from -half_w to CURVE_START_X)
    commands.spawn((
        Sprite::from_color(
            COLOR_ROAD_BORDER,
            Vec2::new(h_straight_len, border_thickness),
        ),
        Transform::from_xyz(h_straight_center_x, half_road, z_marking),
        RoadMarking {
            marking_type: RoadMarkingType::Border,
        },
    ));

    // Bottom border (y = -half_road, from -half_w to CURVE_START_X)
    commands.spawn((
        Sprite::from_color(
            COLOR_ROAD_BORDER,
            Vec2::new(h_straight_len, border_thickness),
        ),
        Transform::from_xyz(h_straight_center_x, -half_road, z_marking),
        RoadMarking {
            marking_type: RoadMarkingType::Border,
        },
    ));

    // West Centerline segment (from -half_w to -half_road)
    let west_seg_len = half_w - half_road;
    let west_center_x = -half_road - (west_seg_len / 2.0);
    commands.spawn((
        Sprite::from_color(
            COLOR_ROAD_CENTERLINE,
            Vec2::new(west_seg_len, centerline_thickness),
        ),
        Transform::from_xyz(west_center_x, 0.0, z_marking),
        RoadMarking {
            marking_type: RoadMarkingType::Centerline,
        },
    ));

    // East Straight Centerline segment (from half_road to CURVE_START_X)
    let east_straight_seg_len = CURVE_START_X - half_road;
    let east_straight_center_x = half_road + (east_straight_seg_len / 2.0);
    commands.spawn((
        Sprite::from_color(
            COLOR_ROAD_CENTERLINE,
            Vec2::new(east_straight_seg_len, centerline_thickness),
        ),
        Transform::from_xyz(east_straight_center_x, 0.0, z_marking),
        RoadMarking {
            marking_type: RoadMarkingType::Centerline,
        },
    ));

    // --- Vertical Road Markings ---
    // Left border (x = -half_road)
    commands.spawn((
        Sprite::from_color(COLOR_ROAD_BORDER, Vec2::new(border_thickness, WORLD_HEIGHT)),
        Transform::from_xyz(-half_road, 0.0, z_marking),
        RoadMarking {
            marking_type: RoadMarkingType::Border,
        },
    ));

    // Right border (x = half_road)
    commands.spawn((
        Sprite::from_color(COLOR_ROAD_BORDER, Vec2::new(border_thickness, WORLD_HEIGHT)),
        Transform::from_xyz(half_road, 0.0, z_marking),
        RoadMarking {
            marking_type: RoadMarkingType::Border,
        },
    ));

    // South Centerline segment (from -half_h to -half_road)
    let south_seg_len = half_h - half_road;
    let south_center_y = -half_road - (south_seg_len / 2.0);
    commands.spawn((
        Sprite::from_color(
            COLOR_ROAD_CENTERLINE,
            Vec2::new(centerline_thickness, south_seg_len),
        ),
        Transform::from_xyz(0.0, south_center_y, z_marking),
        RoadMarking {
            marking_type: RoadMarkingType::Centerline,
        },
    ));

    // North Centerline segment (from half_road to half_h)
    let north_seg_len = half_h - half_road;
    let north_center_y = half_road + (north_seg_len / 2.0);
    commands.spawn((
        Sprite::from_color(
            COLOR_ROAD_CENTERLINE,
            Vec2::new(centerline_thickness, north_seg_len),
        ),
        Transform::from_xyz(0.0, north_center_y, z_marking),
        RoadMarking {
            marking_type: RoadMarkingType::Centerline,
        },
    ));

    // --- Curved Road Markings and Asphalt Segments ---
    let num_curve_steps = 32;
    let d_theta = std::f32::consts::FRAC_PI_2 / (num_curve_steps as f32);
    let asphalt_seg_len = CURVE_RADIUS * d_theta * 1.10;
    let centerline_seg_len = CURVE_RADIUS * d_theta * 1.05;
    let inner_border_radius = CURVE_RADIUS - half_road;
    let outer_border_radius = CURVE_RADIUS + half_road;
    let inner_border_seg_len = inner_border_radius * d_theta * 1.05;
    let outer_border_seg_len = outer_border_radius * d_theta * 1.05;

    for i in 0..num_curve_steps {
        let theta = (i as f32 + 0.5) * d_theta;
        let rot = Quat::from_rotation_z(theta);

        // Asphalt segment
        let pos_asphalt = Vec2::new(
            CURVE_CENTER.x + CURVE_RADIUS * theta.sin(),
            CURVE_CENTER.y - CURVE_RADIUS * theta.cos(),
        );
        commands.spawn((
            Sprite::from_color(COLOR_ROAD_ASPHALT, Vec2::new(asphalt_seg_len, ROAD_WIDTH)),
            Transform::from_xyz(pos_asphalt.x, pos_asphalt.y, Z_ROAD).with_rotation(rot),
        ));

        // Centerline segment
        commands.spawn((
            Sprite::from_color(
                COLOR_ROAD_CENTERLINE,
                Vec2::new(centerline_seg_len, centerline_thickness),
            ),
            Transform::from_xyz(pos_asphalt.x, pos_asphalt.y, z_marking).with_rotation(rot),
            RoadMarking {
                marking_type: RoadMarkingType::Centerline,
            },
        ));

        // Inner border segment
        let pos_inner = Vec2::new(
            CURVE_CENTER.x + inner_border_radius * theta.sin(),
            CURVE_CENTER.y - inner_border_radius * theta.cos(),
        );
        commands.spawn((
            Sprite::from_color(
                COLOR_ROAD_BORDER,
                Vec2::new(inner_border_seg_len, border_thickness),
            ),
            Transform::from_xyz(pos_inner.x, pos_inner.y, z_marking).with_rotation(rot),
            RoadMarking {
                marking_type: RoadMarkingType::Border,
            },
        ));

        // Outer border segment
        let pos_outer = Vec2::new(
            CURVE_CENTER.x + outer_border_radius * theta.sin(),
            CURVE_CENTER.y - outer_border_radius * theta.cos(),
        );
        commands.spawn((
            Sprite::from_color(
                COLOR_ROAD_BORDER,
                Vec2::new(outer_border_seg_len, border_thickness),
            ),
            Transform::from_xyz(pos_outer.x, pos_outer.y, z_marking).with_rotation(rot),
            RoadMarking {
                marking_type: RoadMarkingType::Border,
            },
        ));
    }
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
    fn test_setup_roads_spawns_entities_in_z_road_layer() {
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
            assert_eq!(transform.translation.z, Z_ROAD);
            assert_eq!(road.width, ROAD_WIDTH);
        }

        // Check marking entities
        let mut marking_query = app.world_mut().query::<(&RoadMarking, &Transform)>();
        let markings: Vec<(&RoadMarking, &Transform)> = marking_query.iter(app.world()).collect();
        assert!(!markings.is_empty(), "Should spawn borders and centerlines");

        for (_marking, transform) in markings {
            assert!(
                transform.translation.z >= Z_ROAD && transform.translation.z < Z_CAR,
                "Road markings must be layered below cars (z < Z_CAR)"
            );
        }
        // Check grid line entities
        let mut grid_query = app.world_mut().query::<(&GridLine, &Transform)>();
        let grid_lines: Vec<(&GridLine, &Transform)> = grid_query.iter(app.world()).collect();
        assert!(!grid_lines.is_empty(), "Should spawn background grid lines");
        for (_grid, transform) in grid_lines {
            assert!(
                transform.translation.z < Z_ROAD,
                "Grid lines must be layered behind road asphalt"
            );
        }
    }

    #[test]
    fn test_road_and_marking_exact_coordinates() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<RoadGeometry>()
            .add_systems(Startup, setup_roads);
        app.update();

        // Verify horizontal road at y = 0, vertical road at x = 0
        let mut road_query = app
            .world_mut()
            .query::<(&Road, &Transform, Option<&Sprite>)>();
        let mut found_h = false;
        let mut found_v = false;
        let mut found_c = false;
        for (road, transform, maybe_sprite) in road_query.iter(app.world()) {
            match road.orientation {
                RoadOrientation::Horizontal => {
                    assert_eq!(transform.translation.z, Z_ROAD);
                    assert_eq!(
                        maybe_sprite.unwrap().custom_size,
                        Some(Vec2::new(HALF_WORLD_WIDTH + CURVE_START_X, ROAD_WIDTH))
                    );
                    found_h = true;
                }
                RoadOrientation::Vertical => {
                    assert_eq!(transform.translation, Vec3::new(0.0, 0.0, Z_ROAD));
                    assert_eq!(
                        maybe_sprite.unwrap().custom_size,
                        Some(Vec2::new(ROAD_WIDTH, WORLD_HEIGHT))
                    );
                    found_v = true;
                }
                RoadOrientation::Curved => {
                    assert_eq!(
                        transform.translation,
                        Vec3::new(CURVE_START_X, CURVE_CENTER.y, Z_ROAD)
                    );
                    assert!(
                        maybe_sprite.is_none(),
                        "Curved road parent should not have a stray sprite at the arc center"
                    );
                    found_c = true;
                }
            }
        }
        assert!(found_h && found_v && found_c);

        // Verify borders and centerlines at exact offsets
        let mut marking_query = app.world_mut().query::<(&RoadMarking, &Transform)>();
        let mut border_positions = Vec::new();
        let mut centerline_positions = Vec::new();
        for (marking, transform) in marking_query.iter(app.world()) {
            match marking.marking_type {
                RoadMarkingType::Border => border_positions.push(transform.translation.truncate()),
                RoadMarkingType::Centerline => {
                    centerline_positions.push(transform.translation.truncate())
                }
            }
        }
        assert_eq!(border_positions.len(), 68);
        let h_straight_center_x = (-HALF_WORLD_WIDTH + CURVE_START_X) / 2.0;
        assert!(border_positions.contains(&Vec2::new(h_straight_center_x, 60.0)));
        assert!(border_positions.contains(&Vec2::new(h_straight_center_x, -60.0)));
        assert!(border_positions.contains(&Vec2::new(-60.0, 0.0)));
        assert!(border_positions.contains(&Vec2::new(60.0, 0.0)));

        assert_eq!(centerline_positions.len(), 36);
        let west_center_x = -60.0 - ((HALF_WORLD_WIDTH - 60.0) / 2.0);
        let east_straight_center_x = 60.0 + ((CURVE_START_X - 60.0) / 2.0);
        let south_center_y = -60.0 - ((HALF_WORLD_HEIGHT - 60.0) / 2.0);
        let north_center_y = 60.0 + ((HALF_WORLD_HEIGHT - 60.0) / 2.0);
        assert!(centerline_positions.contains(&Vec2::new(west_center_x, 0.0)));
        assert!(centerline_positions.contains(&Vec2::new(east_straight_center_x, 0.0)));
        assert!(centerline_positions.contains(&Vec2::new(0.0, south_center_y)));
        assert!(centerline_positions.contains(&Vec2::new(0.0, north_center_y)));
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
}
