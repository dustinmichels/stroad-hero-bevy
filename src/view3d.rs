use crate::state::GameSet;

use crate::constants::*;
use crate::road::{
    COLOR_ROAD_ASPHALT, COLOR_ROAD_BORDER, COLOR_ROAD_CENTERLINE, RoadGeometry, RoadPieceKind,
    road_pieces,
};
use crate::{collision, player, sim, traffic};
use bevy::light::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use std::collections::HashMap;
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

/// Marker component for the 3D ground plane.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GroundPlane;

/// Y elevation of the grass ground plane, slightly below asphalt (y = 0.0) to prevent z-fighting.
pub const GROUND_PLANE_Y: f32 = -0.1;

/// Spawns the 3D grass ground plane sized to prevent seeing past the world boundary at pitch angle.
pub fn setup_ground_3d(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let plane_mesh = meshes.add(
        Plane3d::default()
            .mesh()
            .size(GROUND_PLANE_WIDTH, GROUND_PLANE_HEIGHT),
    );
    let plane_material = materials.add(StandardMaterial {
        base_color: COLOR_GROUND_GRASS,
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(plane_material),
        Transform::from_translation(ground_to_3d(Vec2::ZERO, GROUND_PLANE_Y)),
        GroundPlane,
    ));
}

/// Marker component for 3D road visual pieces.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoadPiece3d {
    pub kind: RoadPieceKind,
}

/// Spawns thin flat meshes for road pieces (asphalt at y = 0.0, markings at y = 0.1).
/// Mesh handles are shared per shape (size) and material handles are shared per kind.
pub fn setup_roads_3d(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    road_geom: Option<Res<RoadGeometry>>,
) {
    let geom = road_geom.as_deref().cloned().unwrap_or_default();
    let pieces = road_pieces(&geom);

    let asphalt_mat = materials.add(StandardMaterial {
        base_color: COLOR_ROAD_ASPHALT,
        perceptual_roughness: 0.9,
        ..default()
    });
    let centerline_mat = materials.add(StandardMaterial {
        base_color: COLOR_ROAD_CENTERLINE,
        perceptual_roughness: 0.9,
        ..default()
    });
    let border_mat = materials.add(StandardMaterial {
        base_color: COLOR_ROAD_BORDER,
        perceptual_roughness: 0.9,
        ..default()
    });
    let stop_line_mat = materials.add(StandardMaterial {
        base_color: crate::road::COLOR_ROAD_STOP_LINE,
        perceptual_roughness: 0.9,
        ..default()
    });

    let mut mesh_cache: HashMap<[u32; 2], Handle<Mesh>> = HashMap::new();

    for piece in pieces {
        let (y, mat) = match piece.kind {
            RoadPieceKind::Asphalt => (0.0, asphalt_mat.clone()),
            RoadPieceKind::Centerline => (0.1, centerline_mat.clone()),
            RoadPieceKind::Border => (0.1, border_mat.clone()),
            RoadPieceKind::StopLine => (0.11, stop_line_mat.clone()),
        };

        let key = [piece.size.x.to_bits(), piece.size.y.to_bits()];
        let mesh_handle = mesh_cache
            .entry(key)
            .or_insert_with(|| {
                meshes.add(Plane3d::default().mesh().size(piece.size.x, piece.size.y))
            })
            .clone();

        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(mat),
            Transform::from_translation(ground_to_3d(piece.center, y))
                .with_rotation(heading_to_3d(piece.angle)),
            RoadPiece3d { kind: piece.kind },
        ));
    }
}

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
        bevy::ui::IsDefaultUiCamera,
        AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
            ..default()
        },
    ));

    // Directional sun light with a single shadow cascade for WebGL2 compliance.
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 8000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.5, 0.0)),
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 2000.0,
            ..default()
        }
        .build(),
    ));
}

/// Updates the 3D perspective camera to follow the player, clamped to 2D camera bounds.
type PlayerQueryFilter = (With<player::Player>, Without<ThreeDCamera>);

pub fn camera3d_follow_player(
    player_query: Query<&sim::GroundPos, PlayerQueryFilter>,
    mut camera_query: Query<&mut Transform, (With<ThreeDCamera>, Without<player::Player>)>,
) {
    let Ok(ground_pos) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let raw_pos = ground_pos.0;
    let min_cam_x = WORLD_BOUNDS_MIN.x + HALF_WINDOW_WIDTH;
    let max_cam_x = WORLD_BOUNDS_MAX.x - HALF_WINDOW_WIDTH;
    let min_cam_y = WORLD_BOUNDS_MIN.y + HALF_WINDOW_HEIGHT;
    let max_cam_y = WORLD_BOUNDS_MAX.y - HALF_WINDOW_HEIGHT;

    let target_x = raw_pos.x.clamp(min_cam_x, max_cam_x);
    let target_y = raw_pos.y.clamp(min_cam_y, max_cam_y);

    let target = ground_to_3d(Vec2::new(target_x, target_y), 0.0);
    let cam_offset = Vec3::new(
        0.0,
        CAMERA3D_DISTANCE * CAMERA3D_PITCH.sin(),
        CAMERA3D_DISTANCE * CAMERA3D_PITCH.cos(),
    );

    let new_transform =
        Transform::from_translation(target + cam_offset).looking_at(target, Vec3::Y);
    if *camera_transform != new_transform {
        *camera_transform = new_transform;
    }
}

pub struct View3dPlugin;

impl Plugin for View3dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadGeometry>()
            .init_resource::<CarAssets3d>()
            .init_resource::<SignalAssets3d>()
            .insert_resource(GlobalAmbientLight {
                color: Color::WHITE,
                brightness: 200.0,
                ..default()
            })
            .add_observer(on_add_player)
            .add_observer(on_add_car)
            .add_observer(on_add_blood_splat)
            .add_systems(
                Startup,
                (
                    setup_camera_3d,
                    setup_ground_3d,
                    setup_roads_3d,
                    setup_signals_3d,
                ),
            )
            .add_systems(
                Update,
                (
                    sync_3d_player.after(player::player_movement),
                    sync_3d_cars
                        .after(traffic::car_movement)
                        .after(traffic::car_cleanup),
                    sync_3d_signals.after(traffic::tick_traffic_signal),
                    camera3d_follow_player.after(sync_3d_player),
                )
                    .in_set(GameSet::RenderSync),
            );
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[relationship(relationship_target = View3d)]
pub struct View3dOf(pub Entity);

#[derive(Component, Debug, Default)]
#[relationship_target(relationship = View3dOf, linked_spawn)]
pub struct View3d(Vec<Entity>);

impl std::ops::Deref for View3d {
    type Target = [Entity];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Marker component for the 3D player pill.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Player3d;

/// Marker component for the 3D car box.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Car3d;

/// Marker component for the 3D blood splat decal.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BloodSplat3d;

/// Shared 3D assets for vehicles to avoid per-car asset allocations.
#[derive(Resource, Clone, Debug)]
pub struct CarAssets3d {
    /// Shared cuboid mesh for standard car dimensions (`CAR_LENGTH × CAR_HEIGHT_3D × CAR_WIDTH`).
    pub mesh: Handle<Mesh>,
    /// Small shared palette of materials corresponding to `CAR_COLORS`.
    pub palette: Vec<Handle<StandardMaterial>>,
    /// Shared material for braking cars (bright brake-light red).
    pub braking_material: Handle<StandardMaterial>,
    /// Shared material for crashed cars (crumpled dark charcoal/slate).
    pub crashed_material: Handle<StandardMaterial>,
}

impl CarAssets3d {
    /// Get the shared normal material for a given color index without asset allocation.
    pub fn normal_material(&self, color_index: usize) -> Handle<StandardMaterial> {
        if self.palette.is_empty() {
            Handle::default()
        } else {
            self.palette[color_index % self.palette.len()].clone()
        }
    }

    /// Get the shared braking material without asset allocation.
    pub fn braking_material(&self) -> Handle<StandardMaterial> {
        self.braking_material.clone()
    }

    /// Get the shared crashed material without asset allocation.
    pub fn crashed_material(&self) -> Handle<StandardMaterial> {
        self.crashed_material.clone()
    }

    /// Get the shared material handle corresponding to a vehicle's visual state.
    pub fn material_for_state(
        &self,
        state: traffic::CarVisualState,
        color_index: usize,
    ) -> Handle<StandardMaterial> {
        match state {
            traffic::CarVisualState::Normal => self.normal_material(color_index),
            traffic::CarVisualState::Braking => self.braking_material(),
            traffic::CarVisualState::Crashed => self.crashed_material(),
        }
    }
}

impl FromWorld for CarAssets3d {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh = meshes.add(Cuboid::new(CAR_LENGTH, CAR_HEIGHT_3D, CAR_WIDTH));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let palette = traffic::CAR_COLORS
            .iter()
            .map(|&color| {
                materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.5,
                    ..default()
                })
            })
            .collect();

        let braking_material = materials.add(StandardMaterial {
            base_color: traffic::COLOR_CAR_BRAKING,
            perceptual_roughness: 0.4,
            ..default()
        });

        let crashed_material = materials.add(StandardMaterial {
            base_color: traffic::COLOR_CAR_CRASHED,
            perceptual_roughness: 0.8,
            ..default()
        });

        Self {
            mesh,
            palette,
            braking_material,
            crashed_material,
        }
    }
}

/// Observer that spawns a 3D Cuboid box when a `Car` entity is added.
/// Box dimensions are length × CAR_HEIGHT_3D × width (long axis = local X).
/// Uses shared mesh and palette of materials from `CarAssets3d` with zero per-car allocations.
/// Query data representing simulation state for 3D car visual synchronization.
pub type CarSimVisualQuery<'a> = (
    &'a traffic::Car,
    Option<&'a sim::GroundPos>,
    Option<&'a sim::Heading>,
    Option<&'a traffic::CarMotion>,
    Has<traffic::Crashed>,
);

/// Query data representing the 3D proxy entity for car rendering synchronization.
pub type Car3dViewQuery<'a> = (
    &'a View3dOf,
    &'a mut Transform,
    Option<&'a mut MeshMaterial3d<StandardMaterial>>,
);

pub fn on_add_car(
    trigger: On<Add<traffic::Car>>,
    mut commands: Commands,
    car_assets: Res<CarAssets3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    car_query: Query<CarSimVisualQuery>,
) {
    let car_entity = trigger.entity;
    let (length, width, color_idx, pos, heading, visual_state) = car_query
        .get(car_entity)
        .map(|(car, gp, h, maybe_motion, is_crashed)| {
            let state = maybe_motion
                .map(|m| m.visual_state(is_crashed))
                .unwrap_or(if is_crashed {
                    traffic::CarVisualState::Crashed
                } else {
                    traffic::CarVisualState::Normal
                });
            (
                car.length,
                car.width,
                car.color_index,
                gp.map(|g| g.0).unwrap_or(Vec2::ZERO),
                h.map(|h| h.0).unwrap_or(0.0),
                state,
            )
        })
        .unwrap_or((
            CAR_LENGTH,
            CAR_WIDTH,
            0,
            Vec2::ZERO,
            0.0,
            traffic::CarVisualState::Normal,
        ));

    let half_height = CAR_HEIGHT_3D / 2.0;
    let mesh =
        if (length - CAR_LENGTH).abs() < f32::EPSILON && (width - CAR_WIDTH).abs() < f32::EPSILON {
            car_assets.mesh.clone()
        } else {
            meshes.add(Cuboid::new(length, CAR_HEIGHT_3D, width))
        };

    let material = car_assets.material_for_state(visual_state, color_idx);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(ground_to_3d(pos, half_height))
            .with_rotation(heading_to_3d(heading)),
        View3dOf(car_entity),
        Car3d,
    ));
}

/// Observer that spawns a 3D decal when a `BloodSplat` entity is added.
/// Decal is generated from `create_blood_splat_mesh(seed, …)`, rotated $-\pi/2$ about X
/// to lie flat at $y = 0.05$, using unlit `StandardMaterial`.
pub fn on_add_blood_splat(
    trigger: On<Add<collision::BloodSplat>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    splat_query: Query<(&collision::BloodSplat, Option<&sim::GroundPos>)>,
    mut cached_mat: Local<Option<Handle<StandardMaterial>>>,
) {
    let splat_entity = trigger.entity;
    let (seed, pos) = splat_query
        .get(splat_entity)
        .map(|(splat, gp)| (splat.seed, gp.map(|g| g.0).unwrap_or(Vec2::ZERO)))
        .unwrap_or((0, Vec2::ZERO));

    let mesh = meshes.add(collision::create_blood_splat_mesh(
        seed,
        PLAYER_RADIUS * 1.15,
    ));
    let material = cached_mat
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: COLOR_BLOOD_SPLAT,
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            })
        })
        .clone();

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(ground_to_3d(pos, 0.05))
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        View3dOf(splat_entity),
        BloodSplat3d,
    ));
}

/// Observer that spawns a 3D Capsule pill when a `Player` entity is added.
pub fn on_add_player(
    trigger: On<Add<player::Player>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query: Query<(&sim::GroundPos, Option<&player::Player>)>,
) {
    let player = trigger.entity;
    let (pos, is_alive) = player_query
        .get(player)
        .map(|(gp, p)| (gp.0, p.map(|p| p.alive).unwrap_or(true)))
        .unwrap_or((PLAYER_SPAWN_POS, true));
    let half_height = PLAYER_HEIGHT_3D / 2.0 + PLAYER_RADIUS;
    let mesh = meshes.add(Capsule3d::new(PLAYER_RADIUS, PLAYER_HEIGHT_3D));
    let color = if is_alive {
        player::COLOR_PLAYER
    } else {
        player::COLOR_PLAYER_DEAD
    };
    let material = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(ground_to_3d(pos, half_height)),
        View3dOf(player),
        Player3d,
    ));
}

/// Syncs the 3D player pill position and material color with simulation state.
/// Translation is set to `ground_to_3d(pos, half_height)` so the pill stands flush on the ground plane.
/// Material color dynamically reflects `Player.alive` (`COLOR_PLAYER` / `COLOR_PLAYER_DEAD`).
pub fn sync_3d_player(
    player_query: Query<(&sim::GroundPos, &player::Player)>,
    mut view_query: Query<
        (&View3dOf, &mut Transform, &MeshMaterial3d<StandardMaterial>),
        With<Player3d>,
    >,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let half_height = PLAYER_HEIGHT_3D / 2.0 + PLAYER_RADIUS;
    for (view_of, mut transform, mat_handle) in &mut view_query {
        if let Ok((ground_pos, player)) = player_query.get(view_of.0) {
            let target_pos = ground_to_3d(ground_pos.0, half_height);
            if transform.translation != target_pos {
                transform.translation = target_pos;
            }

            let target_color = if player.alive {
                player::COLOR_PLAYER
            } else {
                player::COLOR_PLAYER_DEAD
            };

            if let Some(mut mat) = materials
                .as_deref_mut()
                .and_then(|m| m.get_mut(&mat_handle.0))
                .filter(|m| m.base_color != target_color)
            {
                mat.base_color = target_color;
            }
        }
    }
}

/// Syncs the 3D car box position, rotation, and driving-state material with simulation state.
/// Translation is set to `ground_to_3d(pos, CAR_HEIGHT_3D / 2.0)` so the car rests flush on the asphalt ground.
/// Rotation is synced from `heading_to_3d(Heading)`.
/// Material is updated from shared palette materials based on `CarVisualState` (`Normal`, `Braking`, `Crashed`)
/// without per-car material allocations or housing simulation decisions in the rendering system.
pub fn sync_3d_cars(
    car_query: Query<CarSimVisualQuery>,
    mut view_query: Query<Car3dViewQuery, With<Car3d>>,
    car_assets: Option<Res<CarAssets3d>>,
) {
    let half_height = CAR_HEIGHT_3D / 2.0;
    for (view_of, mut transform, mut mat_handle) in &mut view_query {
        if let Ok((car, ground_pos, heading, maybe_motion, is_crashed)) = car_query.get(view_of.0) {
            if let Some(gp) = ground_pos {
                let target_pos = ground_to_3d(gp.0, half_height);
                if transform.translation != target_pos {
                    transform.translation = target_pos;
                }
            }
            if let Some(h) = heading {
                let target_rot = heading_to_3d(h.0);
                if transform.rotation != target_rot {
                    transform.rotation = target_rot;
                }
            }
            if let (Some(assets), Some(mat)) = (&car_assets, &mut mat_handle) {
                let visual_state =
                    maybe_motion
                        .map(|m| m.visual_state(is_crashed))
                        .unwrap_or(if is_crashed {
                            traffic::CarVisualState::Crashed
                        } else {
                            traffic::CarVisualState::Normal
                        });
                let target_mat = assets.material_for_state(visual_state, car.color_index);
                if mat.0 != target_mat {
                    mat.0 = target_mat;
                }
            }
        }
    }
}

/// Marker component for 3D signal heads, identifying the approach lane.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalHead3d {
    pub lane: traffic::TrafficLane,
}

/// Marker component for a 3D signal lens indicating which traffic light state it represents.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalLens3d {
    pub state: traffic::TrafficLightState,
}

/// Shared 3D assets for traffic signals to avoid per-signal asset allocations.
#[derive(Resource, Clone, Debug)]
pub struct SignalAssets3d {
    pub pole_mesh: Handle<Mesh>,
    pub housing_mesh: Handle<Mesh>,
    pub lens_mesh: Handle<Mesh>,
    pub pole_material: Handle<StandardMaterial>,
    pub housing_material: Handle<StandardMaterial>,
    pub off_lens_material: Handle<StandardMaterial>,
    pub red_lens_material: Handle<StandardMaterial>,
    pub yellow_lens_material: Handle<StandardMaterial>,
    pub green_lens_material: Handle<StandardMaterial>,
}

impl SignalAssets3d {
    pub fn lens_material(&self, state: traffic::TrafficLightState) -> Handle<StandardMaterial> {
        match state {
            traffic::TrafficLightState::Red => self.red_lens_material.clone(),
            traffic::TrafficLightState::Yellow => self.yellow_lens_material.clone(),
            traffic::TrafficLightState::Green => self.green_lens_material.clone(),
        }
    }
}

impl FromWorld for SignalAssets3d {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let pole_mesh = meshes.add(Cylinder::new(SIGNAL_POLE_RADIUS, SIGNAL_POLE_HEIGHT));
        let housing_mesh = meshes.add(Cuboid::new(
            SIGNAL_HOUSING_DEPTH,
            SIGNAL_HOUSING_HEIGHT,
            SIGNAL_HOUSING_WIDTH,
        ));
        let lens_mesh = meshes.add(Sphere::new(SIGNAL_LENS_RADIUS));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let pole_material = materials.add(StandardMaterial {
            base_color: COLOR_SIGNAL_POLE,
            perceptual_roughness: 0.8,
            ..default()
        });
        let housing_material = materials.add(StandardMaterial {
            base_color: COLOR_SIGNAL_HOUSING,
            perceptual_roughness: 0.8,
            ..default()
        });
        let off_lens_material = materials.add(StandardMaterial {
            base_color: COLOR_SIGNAL_OFF,
            perceptual_roughness: 0.9,
            ..default()
        });
        let red_lens_material = materials.add(StandardMaterial {
            base_color: COLOR_SIGNAL_RED,
            emissive: LinearRgba::from(COLOR_SIGNAL_RED) * 3.0,
            unlit: true,
            ..default()
        });
        let yellow_lens_material = materials.add(StandardMaterial {
            base_color: COLOR_SIGNAL_YELLOW,
            emissive: LinearRgba::from(COLOR_SIGNAL_YELLOW) * 3.0,
            unlit: true,
            ..default()
        });
        let green_lens_material = materials.add(StandardMaterial {
            base_color: COLOR_SIGNAL_GREEN,
            emissive: LinearRgba::from(COLOR_SIGNAL_GREEN) * 3.0,
            unlit: true,
            ..default()
        });

        Self {
            pole_mesh,
            housing_mesh,
            lens_mesh,
            pole_material,
            housing_material,
            off_lens_material,
            red_lens_material,
            yellow_lens_material,
            green_lens_material,
        }
    }
}

/// Spawns four approach-facing 3D signal heads positioned at the curbs, derived from `RoadGeometry`.
pub fn setup_signals_3d(
    mut commands: Commands,
    signal_assets: Res<SignalAssets3d>,
    road_geom: Option<Res<RoadGeometry>>,
) {
    let geom = road_geom.as_deref().cloned().unwrap_or_default();
    for lane in traffic::TrafficLane::ALL {
        let pos_2d = geom.signal_position(lane);
        let heading = geom.signal_facing_heading(lane);
        let base_pos = ground_to_3d(pos_2d, 0.0);
        let head_rot = heading_to_3d(heading);

        let housing_y = SIGNAL_POLE_HEIGHT - 6.0;
        let lens_x = SIGNAL_HOUSING_DEPTH / 2.0 + 0.2;
        let lenses = [
            (traffic::TrafficLightState::Red, housing_y + 5.0),
            (traffic::TrafficLightState::Yellow, housing_y),
            (traffic::TrafficLightState::Green, housing_y - 5.0),
        ];

        commands
            .spawn((
                SignalHead3d { lane },
                Transform::from_translation(base_pos).with_rotation(head_rot),
                Visibility::Inherited,
            ))
            .with_children(|parent| {
                // Pole
                parent.spawn((
                    Mesh3d(signal_assets.pole_mesh.clone()),
                    MeshMaterial3d(signal_assets.pole_material.clone()),
                    Transform::from_xyz(0.0, SIGNAL_POLE_HEIGHT / 2.0, 0.0),
                ));

                // Housing
                parent.spawn((
                    Mesh3d(signal_assets.housing_mesh.clone()),
                    MeshMaterial3d(signal_assets.housing_material.clone()),
                    Transform::from_xyz(0.0, housing_y, 0.0),
                ));

                // Lenses
                for (state, lens_y) in lenses {
                    // Off background lens
                    parent.spawn((
                        Mesh3d(signal_assets.lens_mesh.clone()),
                        MeshMaterial3d(signal_assets.off_lens_material.clone()),
                        Transform::from_xyz(lens_x, lens_y, 0.0),
                    ));

                    // Active emissive lens
                    parent.spawn((
                        SignalLens3d { state },
                        Mesh3d(signal_assets.lens_mesh.clone()),
                        MeshMaterial3d(signal_assets.lens_material(state)),
                        Transform::from_xyz(lens_x + 0.05, lens_y, 0.0),
                        Visibility::Hidden,
                    ));
                }
            });
    }
}

/// Updates the visibility of 3D signal lenses to reflect the shared `TrafficSignal` resource.
pub fn sync_3d_signals(
    signal: Option<Res<traffic::TrafficSignal>>,
    mut lens_query: Query<(&SignalLens3d, &mut Visibility)>,
) {
    let Some(signal) = signal else { return };
    for (lens, mut visibility) in &mut lens_query {
        let new_vis = if lens.state == signal.state {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != new_vis {
            *visibility = new_vis;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    #[test]
    fn test_plane3d_dimensions() {
        use bevy::render::mesh::VertexAttributeValues;
        let mesh = Plane3d::default().mesh().size(10.0, 20.0).build();
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let VertexAttributeValues::Float32x3(pts) = positions {
            let min_x = pts.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let max_x = pts.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
            let min_y = pts.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let max_y = pts.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
            let min_z = pts.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
            let max_z = pts.iter().map(|p| p[2]).fold(f32::NEG_INFINITY, f32::max);
            assert_eq!((min_x, max_x), (-5.0, 5.0));
            assert_eq!((min_y, max_y), (0.0, 0.0));
            assert_eq!((min_z, max_z), (-10.0, 10.0));
        } else {
            panic!("unexpected vertex format");
        }
    }
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

    #[test]
    fn test_setup_camera_3d_spawns_lights() {
        use bevy::light::CascadeShadowConfig;

        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(View3dPlugin);
        app.update();

        // GlobalAmbientLight resource is configured
        let global_ambient = app.world().get_resource::<GlobalAmbientLight>();
        assert!(
            global_ambient.is_some(),
            "GlobalAmbientLight must be present"
        );
        assert!(global_ambient.unwrap().brightness > 0.0);

        // Camera has AmbientLight component
        let mut cam_query = app.world_mut().query::<(&ThreeDCamera, &AmbientLight)>();
        let cam_ambient = cam_query.iter(app.world()).next();
        assert!(
            cam_ambient.is_some(),
            "ThreeDCamera must have AmbientLight component"
        );
        assert!(cam_ambient.unwrap().1.brightness > 0.0);

        // DirectionalLight with single shadow cascade is spawned
        let mut light_query = app
            .world_mut()
            .query::<(&DirectionalLight, &CascadeShadowConfig)>();
        let light_count = light_query.iter(app.world()).count();
        assert_eq!(
            light_count, 1,
            "Exactly one DirectionalLight should be spawned"
        );

        let (dir_light, cascade_config) = light_query.iter(app.world()).next().unwrap();
        assert!(
            dir_light.shadow_maps_enabled,
            "Shadow maps should be enabled"
        );
        assert_eq!(
            cascade_config.bounds.len(),
            1,
            "Must have exactly 1 shadow cascade for WebGL2 compliance"
        );
    }

    #[test]
    fn test_camera3d_follow_player_tracks_and_clamps() {
        let mut app = App::new();
        app.add_systems(Startup, setup_camera_3d)
            .add_systems(Update, camera3d_follow_player);

        let player_entity = app
            .world_mut()
            .spawn((
                player::Player::default(),
                sim::GroundPos(Vec2::new(0.0, 0.0)),
            ))
            .id();

        app.update();

        let cam_offset = Vec3::new(
            0.0,
            CAMERA3D_DISTANCE * CAMERA3D_PITCH.sin(),
            CAMERA3D_DISTANCE * CAMERA3D_PITCH.cos(),
        );

        // Center pos (0, 0)
        {
            let cam_transform = *app
                .world_mut()
                .query_filtered::<&Transform, With<ThreeDCamera>>()
                .single(app.world())
                .unwrap();
            let expected_cam_pos = ground_to_3d(Vec2::ZERO, 0.0) + cam_offset;
            assert!(
                cam_transform.translation.distance(expected_cam_pos) < 1e-4,
                "Camera translation should follow centered player"
            );
        }

        // Move player within unclamped bounds
        let test_pos = Vec2::new(50.0, 80.0);
        app.world_mut()
            .entity_mut(player_entity)
            .insert(sim::GroundPos(test_pos));
        app.update();

        {
            let cam_transform = *app
                .world_mut()
                .query_filtered::<&Transform, With<ThreeDCamera>>()
                .single(app.world())
                .unwrap();
            let expected_cam_pos = ground_to_3d(test_pos, 0.0) + cam_offset;
            assert!(
                cam_transform.translation.distance(expected_cam_pos) < 1e-4,
                "Camera translation should follow unclamped player move"
            );
        }

        // Move player beyond world bounds (clamp should activate)
        let extreme_pos = Vec2::new(WORLD_BOUNDS_MAX.x + 1000.0, WORLD_BOUNDS_MIN.y - 1000.0);
        app.world_mut()
            .entity_mut(player_entity)
            .insert(sim::GroundPos(extreme_pos));
        app.update();

        {
            let cam_transform = *app
                .world_mut()
                .query_filtered::<&Transform, With<ThreeDCamera>>()
                .single(app.world())
                .unwrap();
            let clamped_x = WORLD_BOUNDS_MAX.x - HALF_WINDOW_WIDTH;
            let clamped_y = WORLD_BOUNDS_MIN.y + HALF_WINDOW_HEIGHT;
            let expected_clamped_target = ground_to_3d(Vec2::new(clamped_x, clamped_y), 0.0);
            let expected_cam_pos = expected_clamped_target + cam_offset;
            assert!(
                cam_transform.translation.distance(expected_cam_pos) < 1e-4,
                "Camera translation should be clamped at world boundaries"
            );
        }
    }

    #[test]
    fn test_setup_ground_3d_spawns_ground_plane() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Startup, setup_ground_3d);
        app.update();

        let mut query = app.world_mut().query::<(
            &GroundPlane,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &Transform,
        )>();
        let count = query.iter(app.world()).count();
        assert_eq!(count, 1, "Exactly one GroundPlane should be spawned");

        let (_, mesh_handle, mat_handle, transform) = query.iter(app.world()).next().unwrap();
        assert_eq!(
            transform.translation,
            ground_to_3d(Vec2::ZERO, GROUND_PLANE_Y),
            "Ground plane must be centered at ground origin at GROUND_PLANE_Y"
        );

        let materials = app.world().resource::<Assets<StandardMaterial>>();
        let material = materials.get(&mat_handle.0).expect("Material must exist");
        assert_eq!(
            material.base_color, COLOR_GROUND_GRASS,
            "Ground plane material must have grass color"
        );

        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&mesh_handle.0).expect("Mesh must exist");
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("Mesh must have position attribute")
            .as_float3()
            .expect("Positions must be float3");
        let min_x = positions.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let max_x = positions
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let min_z = positions.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
        let max_z = positions
            .iter()
            .map(|p| p[2])
            .fold(f32::NEG_INFINITY, f32::max);
        let width = max_x - min_x;
        let depth = max_z - min_z;
        assert_eq!(width, GROUND_PLANE_WIDTH);
        assert_eq!(depth, GROUND_PLANE_HEIGHT);
        assert!(
            width >= WORLD_WIDTH,
            "Ground plane width must cover world width"
        );
        assert!(
            depth >= WORLD_HEIGHT,
            "Ground plane depth must cover world height"
        );
    }

    #[test]
    fn test_setup_roads_3d_spawns_pieces_with_shared_resources() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<RoadGeometry>()
            .add_systems(Startup, setup_roads_3d);
        app.update();

        let mut query = app.world_mut().query::<(
            &RoadPiece3d,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &Transform,
        )>();
        let pieces: Vec<(
            &RoadPiece3d,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &Transform,
        )> = query.iter(app.world()).collect();

        // 142 total pieces: 34 asphalt, 36 centerline, 68 border, 4 stop lines
        assert_eq!(pieces.len(), 142, "Must spawn all 142 road pieces");

        let mut unique_materials = std::collections::HashSet::new();
        let mut unique_meshes = std::collections::HashSet::new();

        let mut asphalt_count = 0;
        let mut centerline_count = 0;
        let mut border_count = 0;
        let mut stop_line_count = 0;

        for (road_piece, mesh, mat, transform) in &pieces {
            unique_materials.insert(mat.0.id());
            unique_meshes.insert(mesh.0.id());

            match road_piece.kind {
                RoadPieceKind::Asphalt => {
                    asphalt_count += 1;
                    assert!(
                        (transform.translation.y - 0.0).abs() < 1e-5,
                        "Asphalt y must be 0.0, got {}",
                        transform.translation.y
                    );
                }
                RoadPieceKind::Centerline => {
                    centerline_count += 1;
                    assert!(
                        (transform.translation.y - 0.1).abs() < 1e-5,
                        "Centerline y must be 0.1, got {}",
                        transform.translation.y
                    );
                }
                RoadPieceKind::Border => {
                    border_count += 1;
                    assert!(
                        (transform.translation.y - 0.1).abs() < 1e-5,
                        "Border y must be 0.1, got {}",
                        transform.translation.y
                    );
                }
                RoadPieceKind::StopLine => {
                    stop_line_count += 1;
                    assert!(
                        (transform.translation.y - 0.11).abs() < 1e-5,
                        "Stop line y must be 0.11, got {}",
                        transform.translation.y
                    );
                }
            }
        }

        assert_eq!(asphalt_count, 34);
        assert_eq!(centerline_count, 36);
        assert_eq!(border_count, 68);
        assert_eq!(stop_line_count, 4);

        // Exactly 4 materials: 1 per kind
        assert_eq!(
            unique_materials.len(),
            4,
            "Materials must be shared per kind (exactly 4)"
        );

        // Verify shared meshes: 142 pieces share 13 unique shapes
        assert_eq!(
            unique_meshes.len(),
            13,
            "Meshes must be shared per shape (exactly 13)"
        );
    }

    #[test]
    fn test_on_add_player_spawns_capsule3d_and_links() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_observer(on_add_player);

        let player_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(10.0, 20.0)),
                player::Player::default(),
            ))
            .id();

        // Query for the 3D proxy entity
        let mut proxy_query = app.world_mut().query::<(
            Entity,
            &View3dOf,
            &Transform,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &Player3d,
        )>();
        let (proxy_entity, view_of, transform, _mesh_handle, mat_handle, _) =
            proxy_query.single(app.world()).unwrap();

        assert_eq!(view_of.0, player_entity);

        // Check transform: ground_to_3d((10, 20), half_height)
        let half_height = PLAYER_HEIGHT_3D / 2.0 + PLAYER_RADIUS;
        let expected_pos = ground_to_3d(Vec2::new(10.0, 20.0), half_height);
        assert_eq!(transform.translation, expected_pos);

        // Check material color is COLOR_PLAYER
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        let mat = materials.get(&mat_handle.0).unwrap();
        assert_eq!(mat.base_color, player::COLOR_PLAYER);

        // Verify relationship target on player entity
        let view_target = app.world().get::<View3d>(player_entity).unwrap();
        assert_eq!(view_target.as_ref(), &[proxy_entity]);

        // Verify linked despawn
        app.world_mut().despawn(player_entity);
        assert!(app.world().get_entity(player_entity).is_err());
        assert!(
            app.world().get_entity(proxy_entity).is_err(),
            "3D proxy must be despawned when player is despawned via linked_spawn"
        );
    }

    #[test]
    fn test_sync_3d_player_updates_translation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_observer(on_add_player)
            .add_systems(Update, sync_3d_player);

        let player_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(0.0, 0.0)),
                player::Player::default(),
            ))
            .id();

        app.update();

        let half_height = PLAYER_HEIGHT_3D / 2.0 + PLAYER_RADIUS;

        // Move player
        let new_pos = Vec2::new(125.0, -45.0);
        app.world_mut()
            .get_mut::<sim::GroundPos>(player_entity)
            .unwrap()
            .0 = new_pos;

        app.update();

        let mut proxy_query = app
            .world_mut()
            .query_filtered::<(&View3dOf, &Transform), With<Player3d>>();
        let (view_of, transform) = proxy_query.single(app.world()).unwrap();
        assert_eq!(view_of.0, player_entity);

        let expected_3d = ground_to_3d(new_pos, half_height);
        assert_eq!(transform.translation, expected_3d);
        // Check flush on ground: bottom of pill = y - half_height = 0.0
        assert_eq!(transform.translation.y - half_height, 0.0);
    }

    #[test]
    fn test_player_material_dynamically_updates_on_alive_state() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_observer(on_add_player)
            .add_systems(Update, sync_3d_player);

        let player_entity = app
            .world_mut()
            .spawn((sim::GroundPos(Vec2::ZERO), player::Player::default()))
            .id();

        app.update();

        let mut proxy_query = app
            .world_mut()
            .query_filtered::<&MeshMaterial3d<StandardMaterial>, With<Player3d>>();
        let mat_handle = proxy_query.single(app.world()).unwrap().0.clone();

        // Initially alive -> COLOR_PLAYER
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(
            materials.get(&mat_handle).unwrap().base_color,
            player::COLOR_PLAYER
        );

        // Player dies -> COLOR_PLAYER_DEAD
        app.world_mut()
            .get_mut::<player::Player>(player_entity)
            .unwrap()
            .alive = false;
        app.update();

        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(
            materials.get(&mat_handle).unwrap().base_color,
            player::COLOR_PLAYER_DEAD
        );

        // Player revives -> COLOR_PLAYER
        app.world_mut()
            .get_mut::<player::Player>(player_entity)
            .unwrap()
            .alive = true;
        app.update();

        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(
            materials.get(&mat_handle).unwrap().base_color,
            player::COLOR_PLAYER
        );
    }

    #[test]
    fn test_player_spawned_dead_initializes_with_dead_color() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_observer(on_add_player);

        app.world_mut().spawn((
            sim::GroundPos(Vec2::ZERO),
            player::Player {
                alive: false,
                ..default()
            },
        ));

        let mut proxy_query = app
            .world_mut()
            .query_filtered::<&MeshMaterial3d<StandardMaterial>, With<Player3d>>();
        let mat_handle = proxy_query.single(app.world()).unwrap().0.clone();

        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(
            materials.get(&mat_handle).unwrap().base_color,
            player::COLOR_PLAYER_DEAD
        );
    }

    #[test]
    fn test_on_add_car_spawns_cuboid_and_links() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .add_observer(on_add_car);

        let car_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(100.0, 50.0)),
                sim::Heading::new(std::f32::consts::FRAC_PI_2),
                traffic::Car::new(CAR_SPEED, Vec2::Y, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        // Query for the 3D proxy entity
        let mut proxy_query = app.world_mut().query::<(
            Entity,
            &View3dOf,
            &Transform,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &Car3d,
        )>();
        let (proxy_entity, view_of, transform, mesh_handle, _mat_handle, _) =
            proxy_query.single(app.world()).unwrap();

        assert_eq!(view_of.0, car_entity);

        // Check transform: translation at ground_to_3d((100, 50), CAR_HEIGHT_3D / 2.0)
        let half_height = CAR_HEIGHT_3D / 2.0;
        let expected_pos = ground_to_3d(Vec2::new(100.0, 50.0), half_height);
        assert_eq!(transform.translation, expected_pos);
        // Bottom of car is flush with asphalt ground plane (y = 0.0)
        assert_eq!(transform.translation.y - half_height, 0.0);

        // Check rotation matches heading
        let expected_rot = heading_to_3d(std::f32::consts::FRAC_PI_2);
        assert_eq!(transform.rotation, expected_rot);

        // Check mesh size: length = CAR_LENGTH, height = CAR_HEIGHT_3D, width = CAR_WIDTH
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&mesh_handle.0).unwrap();
        // Cuboid generates vertex positions; check extents
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            let max_x = positions.iter().map(|p| p[0]).fold(0.0f32, f32::max);
            let max_y = positions.iter().map(|p| p[1]).fold(0.0f32, f32::max);
            let max_z = positions.iter().map(|p| p[2]).fold(0.0f32, f32::max);
            assert_eq!(max_x, CAR_LENGTH / 2.0);
            assert_eq!(max_y, CAR_HEIGHT_3D / 2.0);
            assert_eq!(max_z, CAR_WIDTH / 2.0);
        } else {
            panic!("Mesh has no position attribute");
        }

        // Verify relationship target on car entity
        let view_target = app.world().get::<View3d>(car_entity).unwrap();
        assert_eq!(view_target.as_ref(), &[proxy_entity]);
        // Verify mesh handle is the shared mesh from CarAssets3d
        let car_assets = app.world().resource::<CarAssets3d>();
        assert_eq!(mesh_handle.0, car_assets.mesh);

        // Verify material handle is from CarAssets3d palette
        assert_eq!(_mat_handle.0, car_assets.palette[0]);

        // Verify linked despawn
        app.world_mut().despawn(car_entity);
        assert!(app.world().get_entity(car_entity).is_err());
        assert!(
            app.world().get_entity(proxy_entity).is_err(),
            "3D proxy must be despawned when car is despawned via linked_spawn"
        );
    }

    #[test]
    fn test_car_assets_3d_shared_zero_allocation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .add_observer(on_add_car);

        let car_assets = app.world().resource::<CarAssets3d>().clone();
        assert_eq!(car_assets.palette.len(), traffic::CAR_COLORS.len());

        let mesh_count_before = app.world().resource::<Assets<Mesh>>().len();
        let mat_count_before = app.world().resource::<Assets<StandardMaterial>>().len();

        // Initial allocation: 1 cuboid mesh, 5 palette materials + 1 braking + 1 crashed
        assert_eq!(mesh_count_before, 1);
        assert_eq!(mat_count_before, traffic::CAR_COLORS.len() + 2);
        // Spawn 20 cars with varying color indices
        for i in 0..20 {
            app.world_mut().spawn((
                sim::GroundPos(Vec2::new(i as f32 * 10.0, 0.0)),
                sim::Heading::ZERO,
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_color_index(i),
                traffic::CarMotion::new(CAR_SPEED),
            ));
        }

        // Verify zero asset allocations occurred per car
        let mesh_count_after = app.world().resource::<Assets<Mesh>>().len();
        let mat_count_after = app.world().resource::<Assets<StandardMaterial>>().len();
        assert_eq!(mesh_count_after, mesh_count_before);
        assert_eq!(mat_count_after, mat_count_before);

        // Check every 3D car proxy uses the shared mesh and corresponding palette material
        let mut proxy_query = app.world_mut().query::<(
            &View3dOf,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &Car3d,
        )>();
        for (view_of, mesh3d, mat3d, _) in proxy_query.iter(app.world()) {
            assert_eq!(mesh3d.0, car_assets.mesh);
            let car = app.world().get::<traffic::Car>(view_of.0).unwrap();
            let expected_mat = &car_assets.palette[car.color_index % car_assets.palette.len()];
            assert_eq!(&mat3d.0, expected_mat);
        }
    }

    #[test]
    fn test_car_assets_shared_normal_braking_crashed_appearance_transition() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .add_observer(on_add_car);

        let car_assets = app.world().resource::<CarAssets3d>().clone();
        let materials = app.world().resource::<Assets<StandardMaterial>>();

        // Verify braking material properties
        let braking_mat = materials
            .get(&car_assets.braking_material)
            .expect("braking mat exists");
        assert_eq!(braking_mat.base_color, traffic::COLOR_CAR_BRAKING);

        // Verify crashed material properties
        let crashed_mat = materials
            .get(&car_assets.crashed_material)
            .expect("crashed mat exists");
        assert_eq!(crashed_mat.base_color, traffic::COLOR_CAR_CRASHED);

        let initial_mat_count = materials.len();

        // Spawn car
        let car_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::ZERO),
                sim::Heading::ZERO,
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_color_index(2),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        let view_entity = app.world().get::<View3d>(car_entity).unwrap()[0];

        // 1. Initial normal appearance
        let current_mat = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap()
            .0
            .clone();
        assert_eq!(current_mat, car_assets.normal_material(2));

        // 2. Transition to braking appearance
        *app.world_mut()
            .get_mut::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap() = MeshMaterial3d(car_assets.braking_material());
        let current_mat = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap()
            .0
            .clone();
        assert_eq!(current_mat, car_assets.braking_material);

        // 3. Transition to crashed appearance
        *app.world_mut()
            .get_mut::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap() = MeshMaterial3d(car_assets.crashed_material());
        let current_mat = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap()
            .0
            .clone();
        assert_eq!(current_mat, car_assets.crashed_material);

        // 4. Return to normal appearance
        *app.world_mut()
            .get_mut::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap() = MeshMaterial3d(car_assets.normal_material(2));
        let current_mat = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap()
            .0
            .clone();
        assert_eq!(current_mat, car_assets.normal_material(2));

        // Verify zero new materials were allocated across all state transitions
        let final_mat_count = app.world().resource::<Assets<StandardMaterial>>().len();
        assert_eq!(final_mat_count, initial_mat_count);
    }

    #[test]
    fn test_sync_3d_cars_updates_translation_and_rotation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .add_observer(on_add_car)
            .add_systems(Update, sync_3d_cars);

        let car_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(0.0, 0.0)),
                sim::Heading::new(0.0),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        app.update();

        let mut proxy_query = app.world_mut().query_filtered::<&Transform, With<Car3d>>();
        let initial_transform = *proxy_query.single(app.world()).unwrap();

        let half_height = CAR_HEIGHT_3D / 2.0;
        assert_eq!(
            initial_transform.translation,
            ground_to_3d(Vec2::ZERO, half_height)
        );
        assert_eq!(initial_transform.rotation, heading_to_3d(0.0));

        // Mutate car's GroundPos and Heading
        let new_pos = Vec2::new(175.0, -95.0);
        let new_angle = 1.35;
        app.world_mut()
            .get_mut::<sim::GroundPos>(car_entity)
            .unwrap()
            .0 = new_pos;
        app.world_mut()
            .get_mut::<sim::Heading>(car_entity)
            .unwrap()
            .0 = new_angle;

        app.update();

        let updated_transform = *proxy_query.single(app.world()).unwrap();
        assert_eq!(
            updated_transform.translation,
            ground_to_3d(new_pos, half_height)
        );
        assert_eq!(updated_transform.rotation, heading_to_3d(new_angle));
        // Bottom of car box stays flush on the asphalt ground plane (y = 0.0)
        assert_eq!(updated_transform.translation.y - half_height, 0.0);
    }

    #[test]
    fn test_cars_traverse_straight_and_curve_with_smooth_transitions() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .init_resource::<RoadGeometry>()
            .add_observer(on_add_car)
            .add_systems(Update, (traffic::car_movement, sync_3d_cars).chain());

        let geom = RoadGeometry::default();
        let east_lane_y = geom.east_lane_y();
        let curve_start_x = geom.curve_start_x;
        let half_height = CAR_HEIGHT_3D / 2.0;

        // Spawn Eastbound car before curve
        let start_x = curve_start_x - 50.0;
        let car_e = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(start_x, east_lane_y)),
                sim::Heading::ZERO,
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH)
                    .with_lane(traffic::TrafficLane::Eastbound),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        // Spawn Westbound car before curve (entering from north)
        let start_y = geom.curve_center.y + 50.0;
        let car_w = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(geom.curve_west_lane_x(), start_y)),
                sim::Heading::new(-FRAC_PI_2),
                traffic::Car::new(CAR_SPEED, -Vec2::Y, CAR_LENGTH, CAR_WIDTH)
                    .with_lane(traffic::TrafficLane::Westbound),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        app.update();

        // Step forward in time through straight segment, curve, and north exit
        let dt = 0.05;
        let mut prev_theta = 0.0;

        for _ in 0..85 {
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(std::time::Duration::from_secs_f32(dt));
            }
            app.update();

            let sim_pos = app.world().get::<sim::GroundPos>(car_e).unwrap().0;
            let heading = app.world().get::<sim::Heading>(car_e).unwrap().0;

            // Query proxy transform
            let view_target = app.world().get::<View3d>(car_e).unwrap();
            let proxy = view_target[0];
            let transform = *app.world().get::<Transform>(proxy).unwrap();

            // Translation strictly matches simulation ground pos and flush height
            assert_eq!(transform.translation, ground_to_3d(sim_pos, half_height));
            assert!((transform.translation.y - half_height).abs() < 1e-4);

            // Rotation strictly matches heading_to_3d(heading)
            let expected_rot = heading_to_3d(heading);
            assert!(
                (transform.rotation.to_euler(EulerRot::YXZ).0
                    - expected_rot.to_euler(EulerRot::YXZ).0)
                    .abs()
                    < 1e-4
            );

            // Heading transitions smoothly (monotonically non-decreasing for eastbound curve)
            assert!(heading >= prev_theta - 1e-4);
            assert!(heading <= FRAC_PI_2 + 1e-4);
            // Step delta is bounded (smooth transition, no sudden flip)
            assert!(
                (heading - prev_theta) < (CAR_SPEED / geom.curve_east_lane_radius() * dt + 1e-3)
            );

            prev_theta = heading;

            // Check westbound car proxy transform
            let sim_pos_w = app.world().get::<sim::GroundPos>(car_w).unwrap().0;
            let heading_w = app.world().get::<sim::Heading>(car_w).unwrap().0;
            let view_target_w = app.world().get::<View3d>(car_w).unwrap();
            let proxy_w = view_target_w[0];
            let transform_w = *app.world().get::<Transform>(proxy_w).unwrap();
            assert_eq!(
                transform_w.translation,
                ground_to_3d(sim_pos_w, half_height)
            );
            let expected_rot_w = heading_to_3d(heading_w);
            assert!(
                (transform_w.rotation.to_euler(EulerRot::YXZ).0
                    - expected_rot_w.to_euler(EulerRot::YXZ).0)
                    .abs()
                    < 1e-4
            );
        }

        // After 85 steps (4.25 seconds at 240 px/s = 1020 px traveled):
        // Eastbound car has cleared the curve and is heading North
        let final_heading = app.world().get::<sim::Heading>(car_e).unwrap().0;
        assert!((final_heading - FRAC_PI_2).abs() < 1e-4);
        // Westbound car has cleared the curve and is heading West
        let final_heading_w = app.world().get::<sim::Heading>(car_w).unwrap().0;
        assert!((final_heading_w - PI).abs() < 1e-4);
        let final_pos_w = app.world().get::<sim::GroundPos>(car_w).unwrap().0;
        assert!(final_pos_w.x < geom.curve_start_x);
        assert_eq!(final_pos_w.y, geom.west_lane_y());
    }

    #[test]
    fn test_car_despawn_and_game_restart_removes_3d_boxes_via_linked_spawn() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_observer(on_add_car)
            .add_systems(
                Update,
                (traffic::car_cleanup, crate::collision::restart_game),
            );

        // Part 1: Verify car despawn via car_cleanup removes associated 3D boxes
        let despawn_x = HALF_WORLD_WIDTH + DESPAWN_MARGIN;

        // In-bounds car
        let car_in_bounds = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(0.0, 0.0)),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        // Out-of-bounds car (eastbound past boundary)
        let car_out_of_bounds = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(despawn_x + 50.0, 0.0)),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        // Check both have 3D proxy entities
        let view_in = app.world().get::<View3d>(car_in_bounds).unwrap()[0];
        let view_out = app.world().get::<View3d>(car_out_of_bounds).unwrap()[0];
        assert!(app.world().get_entity(view_in).is_ok());
        assert!(app.world().get_entity(view_out).is_ok());

        let car3d_count_before = app
            .world_mut()
            .query_filtered::<Entity, With<Car3d>>()
            .iter(app.world())
            .count();
        assert_eq!(car3d_count_before, 2);

        // Run car_cleanup system
        app.update();

        // Out-of-bounds car and its 3D box must be despawned
        assert!(app.world().get_entity(car_out_of_bounds).is_err());
        assert!(
            app.world().get_entity(view_out).is_err(),
            "3D car box must be despawned when out-of-bounds car is despawned via linked_spawn"
        );

        // In-bounds car and its 3D box must still exist
        assert!(app.world().get_entity(car_in_bounds).is_ok());
        assert!(app.world().get_entity(view_in).is_ok());

        let car3d_count_after_cleanup = app
            .world_mut()
            .query_filtered::<Entity, With<Car3d>>()
            .iter(app.world())
            .count();
        assert_eq!(car3d_count_after_cleanup, 1);

        // Part 2: Spawn additional cars and verify game restart clears all 3D boxes
        for i in 0..5 {
            app.world_mut().spawn((
                sim::GroundPos(Vec2::new(i as f32 * 30.0, 0.0)),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ));
        }

        let total_cars_before_restart = app
            .world_mut()
            .query::<&traffic::Car>()
            .iter(app.world())
            .count();
        let total_3d_boxes_before_restart = app
            .world_mut()
            .query_filtered::<Entity, With<Car3d>>()
            .iter(app.world())
            .count();
        assert_eq!(total_cars_before_restart, 6);
        assert_eq!(total_3d_boxes_before_restart, 6);

        // Trigger restart via R key
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyR);
        }

        app.update();

        let total_cars_after_restart = app
            .world_mut()
            .query::<&traffic::Car>()
            .iter(app.world())
            .count();
        let total_3d_boxes_after_restart = app
            .world_mut()
            .query_filtered::<Entity, With<Car3d>>()
            .iter(app.world())
            .count();
        assert_eq!(
            total_cars_after_restart, 0,
            "All simulation cars must be despawned on restart"
        );
        assert_eq!(
            total_3d_boxes_after_restart, 0,
            "All 3D car boxes must be despawned when cars are cleared on game restart via linked_spawn"
        );
    }

    #[test]
    fn test_traffic_decisions_remain_in_simulation_data_views_only_render() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .init_resource::<RoadGeometry>()
            .add_observer(on_add_car)
            .add_systems(Update, (traffic::car_movement, sync_3d_cars).chain());

        let geom = RoadGeometry::default();
        let initial_pos = Vec2::new(0.0, geom.east_lane_y());
        let initial_speed = 150.0;
        let car_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(initial_pos),
                sim::Heading::ZERO,
                traffic::Car::new(initial_speed, Vec2::X, CAR_LENGTH, CAR_WIDTH)
                    .with_lane(traffic::TrafficLane::Eastbound),
                traffic::CarMotion::new(initial_speed),
            ))
            .id();

        app.update();

        let view_entity = app.world().get::<View3d>(car_entity).unwrap()[0];
        let half_height = CAR_HEIGHT_3D / 2.0;

        // View entity should reflect initial simulation position
        let initial_transform = *app.world().get::<Transform>(view_entity).unwrap();
        assert_eq!(
            initial_transform.translation,
            ground_to_3d(initial_pos, half_height)
        );
        assert_eq!(initial_transform.rotation, heading_to_3d(0.0));

        // Advance simulation time
        let dt = 0.5;
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs_f32(dt));

        app.update();

        // Simulation state must have advanced based on Car.speed * dt
        let sim_pos = app.world().get::<sim::GroundPos>(car_entity).unwrap().0;
        let sim_heading = app.world().get::<sim::Heading>(car_entity).unwrap().0;
        let sim_car = app.world().get::<traffic::Car>(car_entity).unwrap();
        let expected_x = initial_pos.x + initial_speed * dt;
        assert_eq!(sim_pos.x, expected_x);
        assert_eq!(sim_pos.y, geom.east_lane_y());
        assert_eq!(sim_heading, 0.0);
        assert_eq!(sim_car.speed, initial_speed);
        assert_eq!(sim_car.lane, Some(traffic::TrafficLane::Eastbound));

        // 3D View must reflect the updated simulation data, not drive it
        let updated_transform = *app.world().get::<Transform>(view_entity).unwrap();
        assert_eq!(
            updated_transform.translation,
            ground_to_3d(sim_pos, half_height)
        );
        assert_eq!(updated_transform.rotation, heading_to_3d(sim_heading));

        // Verify views never write back into simulation data: if a view transform is tampered with,
        // the next sync_3d_cars overwrites the view from simulation data without altering simulation state.
        let mut view_transform = app.world_mut().get_mut::<Transform>(view_entity).unwrap();
        view_transform.translation = Vec3::new(999.0, 999.0, 999.0);

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.0));
        // Run only sync_3d_cars
        let mut sync_app = App::new();
        sync_app.add_systems(Update, sync_3d_cars);
        // Running sync_3d_cars again in `app`:
        app.update();

        let restored_view_transform = *app.world().get::<Transform>(view_entity).unwrap();
        assert_eq!(
            restored_view_transform.translation,
            ground_to_3d(sim_pos, half_height)
        );
        let sim_pos_after = app.world().get::<sim::GroundPos>(car_entity).unwrap().0;
        assert_eq!(sim_pos_after, sim_pos);
    }

    #[test]
    fn test_on_add_blood_splat_spawns_decal_and_links() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_observer(on_add_blood_splat);

        let splat_pos = Vec2::new(45.0, -120.0);
        let splat_seed = 987654321;
        let splat_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(splat_pos),
                collision::BloodSplat { seed: splat_seed },
            ))
            .id();

        // Query for the 3D proxy entity
        let mut proxy_query = app.world_mut().query::<(
            Entity,
            &View3dOf,
            &Transform,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &BloodSplat3d,
        )>();
        let (proxy_entity, view_of, transform, mesh_handle, mat_handle, _) =
            proxy_query.single(app.world()).unwrap();

        assert_eq!(view_of.0, splat_entity);
        assert_ne!(proxy_entity, splat_entity);

        // Check transform: rotated -pi/2 about X to lie flat at y = 0.05
        let expected_pos = ground_to_3d(splat_pos, 0.05);
        assert_eq!(transform.translation, expected_pos);
        assert_eq!(transform.translation.y, 0.05);

        let expected_rot = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        assert!(
            transform.rotation.abs_diff_eq(expected_rot, 1e-6),
            "Splat rotation must be -pi/2 about X"
        );

        // Check material: unlit StandardMaterial with COLOR_BLOOD_SPLAT
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        let mat = materials.get(&mat_handle.0).expect("Material must exist");
        assert_eq!(mat.base_color, COLOR_BLOOD_SPLAT);
        assert!(mat.unlit, "Blood splat material must be unlit");
        assert_eq!(mat.alpha_mode, AlphaMode::Blend);

        // Check mesh matches create_blood_splat_mesh
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&mesh_handle.0).expect("Mesh must exist");
        let expected_mesh = collision::create_blood_splat_mesh(splat_seed, PLAYER_RADIUS * 1.15);
        let pos1 = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let pos2 = expected_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        assert_eq!(pos1.len(), pos2.len());
    }

    #[test]
    fn test_blood_splat_decals_reuse_shared_material() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_observer(on_add_blood_splat);

        for i in 0..5 {
            app.world_mut().spawn((
                sim::GroundPos(Vec2::new(i as f32 * 10.0, 0.0)),
                collision::BloodSplat { seed: i + 100 },
            ));
        }

        let mat_count = app.world().resource::<Assets<StandardMaterial>>().len();
        assert_eq!(
            mat_count, 1,
            "Multiple blood splats must reuse a single shared StandardMaterial asset"
        );
    }

    #[test]
    fn test_blood_splat_despawn_and_game_restart_removes_3d_decals_via_linked_spawn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_observer(on_add_blood_splat)
            .add_systems(Update, crate::collision::restart_game);

        // Spawn player (needed for restart_game query)
        let _player = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::ZERO),
                player::Player {
                    alive: true,
                    ..default()
                },
            ))
            .id();

        // Spawn 3 blood splats
        let splat1 = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(10.0, 20.0)),
                collision::BloodSplat { seed: 101 },
            ))
            .id();
        let splat2 = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(-30.0, 40.0)),
                collision::BloodSplat { seed: 102 },
            ))
            .id();
        let splat3 = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(50.0, -60.0)),
                collision::BloodSplat { seed: 103 },
            ))
            .id();

        // Verify all 3 splats have 3D decal entities linked via View3d
        let view1 = app
            .world()
            .get::<View3d>(splat1)
            .expect("View3d must exist")[0];
        let view2 = app
            .world()
            .get::<View3d>(splat2)
            .expect("View3d must exist")[0];
        let view3 = app
            .world()
            .get::<View3d>(splat3)
            .expect("View3d must exist")[0];

        assert!(app.world().get_entity(view1).is_ok());
        assert!(app.world().get_entity(view2).is_ok());
        assert!(app.world().get_entity(view3).is_ok());

        let splat3d_count_before = app
            .world_mut()
            .query_filtered::<Entity, With<BloodSplat3d>>()
            .iter(app.world())
            .count();
        assert_eq!(splat3d_count_before, 3);

        // Part 1: Verify manual despawn of an individual splat despawns its 3D decal via linked_spawn
        app.world_mut().commands().entity(splat1).despawn();
        app.update();

        assert!(app.world().get_entity(splat1).is_err());
        assert!(
            app.world().get_entity(view1).is_err(),
            "3D splat decal must be despawned when splat entity is despawned via linked_spawn"
        );
        let splat3d_count_after_single_despawn = app
            .world_mut()
            .query_filtered::<Entity, With<BloodSplat3d>>()
            .iter(app.world())
            .count();
        assert_eq!(splat3d_count_after_single_despawn, 2);

        // Part 2: Verify game restart (KeyR) cleans up all remaining splats and their 3D decals
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyR);
        }
        app.update();

        let remaining_splats = app
            .world_mut()
            .query::<&collision::BloodSplat>()
            .iter(app.world())
            .count();
        let remaining_3d_decals = app
            .world_mut()
            .query_filtered::<Entity, With<BloodSplat3d>>()
            .iter(app.world())
            .count();

        assert_eq!(
            remaining_splats, 0,
            "All simulation splats must be cleared on restart"
        );
        assert_eq!(
            remaining_3d_decals, 0,
            "All 3D splat decals must be despawned when game restarts via linked_spawn"
        );
        assert!(app.world().get_entity(view2).is_err());
        assert!(app.world().get_entity(view3).is_err());

        // Part 3: Test game restart during GameOver state via Space key
        let mut app_game_over = App::new();
        app_game_over
            .add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<crate::state::AppState>()
            .add_observer(on_add_blood_splat)
            .add_systems(Update, crate::collision::restart_game);

        app_game_over
            .world_mut()
            .resource_mut::<NextState<crate::state::AppState>>()
            .set(crate::state::AppState::GameOver);
        app_game_over.update();

        let _p = app_game_over
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::ZERO),
                player::Player {
                    alive: false,
                    ..default()
                },
            ))
            .id();
        let s = app_game_over
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::new(10.0, 10.0)),
                collision::BloodSplat { seed: 999 },
            ))
            .id();
        let s_view = app_game_over.world().get::<View3d>(s).unwrap()[0];

        assert_eq!(
            app_game_over
                .world_mut()
                .query_filtered::<Entity, With<BloodSplat3d>>()
                .iter(app_game_over.world())
                .count(),
            1
        );

        {
            let mut keys = app_game_over
                .world_mut()
                .resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Space);
        }
        app_game_over.update();

        assert_eq!(
            app_game_over
                .world_mut()
                .query_filtered::<Entity, With<BloodSplat3d>>()
                .iter(app_game_over.world())
                .count(),
            0,
            "Space key on GameOver state must clear 3D blood splats"
        );
        assert!(app_game_over.world().get_entity(s_view).is_err());
    }

    #[test]
    fn test_setup_signals_3d_spawns_four_approach_heads() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<RoadGeometry>()
            .init_resource::<SignalAssets3d>()
            .add_systems(Startup, setup_signals_3d);

        app.update();

        let geom = RoadGeometry::default();
        let mut heads_query = app.world_mut().query::<(&SignalHead3d, &Transform)>();

        let heads: Vec<(&SignalHead3d, &Transform)> = heads_query.iter(app.world()).collect();
        assert_eq!(heads.len(), 4, "Must spawn exactly 4 approach signal heads");

        for lane in traffic::TrafficLane::ALL {
            let matching = heads.iter().find(|(h, _)| h.lane == lane);
            assert!(
                matching.is_some(),
                "Signal head for lane {:?} must exist",
                lane
            );
            let (_, transform) = matching.unwrap();

            let expected_pos = ground_to_3d(geom.signal_position(lane), 0.0);
            let expected_rot = heading_to_3d(geom.signal_facing_heading(lane));

            assert!(
                transform.translation.distance(expected_pos) < 1e-4,
                "Signal head {:?} translation {:?} must match derived position {:?}",
                lane,
                transform.translation,
                expected_pos
            );
            let forward = transform.rotation * Vec3::X;
            let expected_forward = expected_rot * Vec3::X;
            assert!(
                forward.distance(expected_forward) < 1e-4,
                "Signal head {:?} forward {:?} must face oncoming traffic {:?}",
                lane,
                forward,
                expected_forward
            );
        }
    }

    #[test]
    fn test_signal_assets_3d_shared_zero_allocation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<RoadGeometry>()
            .init_resource::<SignalAssets3d>()
            .add_systems(Startup, setup_signals_3d);

        let mesh_count_before = app.world().resource::<Assets<Mesh>>().len();
        let mat_count_before = app.world().resource::<Assets<StandardMaterial>>().len();

        // Exactly 3 primitive meshes (pole, housing, lens) and 6 materials (pole, housing, off, red, yellow, green)
        assert_eq!(mesh_count_before, 3);
        assert_eq!(mat_count_before, 6);

        app.update();

        let mesh_count_after = app.world().resource::<Assets<Mesh>>().len();
        let mat_count_after = app.world().resource::<Assets<StandardMaterial>>().len();

        // Zero new meshes or materials allocated when spawning the 4 signal heads
        assert_eq!(mesh_count_after, mesh_count_before);
        assert_eq!(mat_count_after, mat_count_before);

        // Verify lens materials have emissive colors configured
        let signal_assets = app.world().resource::<SignalAssets3d>();
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        let red_mat = materials.get(&signal_assets.red_lens_material).unwrap();
        let yellow_mat = materials.get(&signal_assets.yellow_lens_material).unwrap();
        let green_mat = materials.get(&signal_assets.green_lens_material).unwrap();

        assert!(red_mat.emissive.red > 0.0);
        assert!(yellow_mat.emissive.red > 0.0 && yellow_mat.emissive.green > 0.0);
        assert!(green_mat.emissive.green > 0.0);
    }

    #[test]
    fn test_sync_3d_signals_drives_all_heads_without_material_allocation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<RoadGeometry>()
            .init_resource::<SignalAssets3d>()
            .init_resource::<traffic::TrafficSignal>()
            .add_systems(Startup, setup_signals_3d)
            .add_systems(Update, sync_3d_signals);

        app.update();

        let mat_count_initial = app.world().resource::<Assets<StandardMaterial>>().len();

        // Helper to check lens counts by visibility
        let count_visible_lenses =
            |world: &mut World, state: traffic::TrafficLightState| -> usize {
                let mut query = world.query::<(&SignalLens3d, &Visibility)>();
                query
                    .iter(world)
                    .filter(|(lens, vis)| lens.state == state && **vis == Visibility::Inherited)
                    .count()
            };

        // 1. Initial state: Green
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Green),
            4
        );
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Yellow),
            0
        );
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Red),
            0
        );

        // 2. Transition to Yellow
        app.world_mut()
            .resource_mut::<traffic::TrafficSignal>()
            .state = traffic::TrafficLightState::Yellow;
        app.update();

        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Green),
            0
        );
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Yellow),
            4
        );
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Red),
            0
        );

        // 3. Transition to Red
        app.world_mut()
            .resource_mut::<traffic::TrafficSignal>()
            .state = traffic::TrafficLightState::Red;
        app.update();

        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Green),
            0
        );
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Yellow),
            0
        );
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Red),
            4
        );

        // 4. Transition back to Green
        app.world_mut()
            .resource_mut::<traffic::TrafficSignal>()
            .state = traffic::TrafficLightState::Green;
        app.update();

        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Green),
            4
        );
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Yellow),
            0
        );
        assert_eq!(
            count_visible_lenses(app.world_mut(), traffic::TrafficLightState::Red),
            0
        );

        // Verify zero new materials were allocated across transitions
        let mat_count_final = app.world().resource::<Assets<StandardMaterial>>().len();
        assert_eq!(mat_count_final, mat_count_initial);
    }

    #[test]
    fn test_sync_3d_cars_drives_driving_state_materials_without_allocation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .add_observer(on_add_car)
            .add_systems(Update, sync_3d_cars);

        let car_assets = app.world().resource::<CarAssets3d>().clone();
        let initial_mat_count = app.world().resource::<Assets<StandardMaterial>>().len();

        // Spawn car with color index 1 (Royal Blue) at cruise speed
        let car_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(Vec2::ZERO),
                sim::Heading::ZERO,
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH).with_color_index(1),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        app.update();

        let view_entity = app.world().get::<View3d>(car_entity).unwrap()[0];

        // 1. Initial state: Cruising -> normal palette material
        let mat = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap()
            .0
            .clone();
        assert_eq!(mat, car_assets.normal_material(1));

        // 2. Transition to braking state (desired_speed < current_speed)
        app.world_mut()
            .get_mut::<traffic::CarMotion>(car_entity)
            .unwrap()
            .desired_speed = 0.0;
        app.update();

        let mat = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap()
            .0
            .clone();
        assert_eq!(mat, car_assets.braking_material());

        // 3. Transition to crashed state (Crashed component inserted)
        app.world_mut()
            .entity_mut(car_entity)
            .insert(traffic::Crashed::default());
        app.update();

        let mat = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap()
            .0
            .clone();
        assert_eq!(mat, car_assets.crashed_material());

        // 4. Return to normal state (Crashed removed, motion cruising)
        let mut entity_mut = app.world_mut().entity_mut(car_entity);
        entity_mut.remove::<traffic::Crashed>();
        let mut motion = entity_mut.get_mut::<traffic::CarMotion>().unwrap();
        *motion = traffic::CarMotion::new(CAR_SPEED);
        app.update();

        let mat = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(view_entity)
            .unwrap()
            .0
            .clone();
        assert_eq!(mat, car_assets.normal_material(1));

        // 5. Zero additional material allocations throughout all driving state transitions
        let final_mat_count = app.world().resource::<Assets<StandardMaterial>>().len();
        assert_eq!(final_mat_count, initial_mat_count);
    }

    #[test]
    fn test_linking_car_spawn_produces_exactly_one_view3d_proxy_matching_ground_to_3d_and_despawn_removes_proxy()
     {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<CarAssets3d>()
            .add_observer(on_add_car)
            .add_systems(Update, sync_3d_cars);

        let car_pos = Vec2::new(175.0, -42.5);
        let car_heading = 1.25;
        let half_height = CAR_HEIGHT_3D / 2.0;

        // Spawning a car
        let car_entity = app
            .world_mut()
            .spawn((
                sim::GroundPos(car_pos),
                sim::Heading::new(car_heading),
                traffic::Car::new(CAR_SPEED, Vec2::X, CAR_LENGTH, CAR_WIDTH),
                traffic::CarMotion::new(CAR_SPEED),
            ))
            .id();

        app.update();

        // Verify exactly one View3d proxy was created on the car entity
        let view_target = app
            .world()
            .get::<View3d>(car_entity)
            .expect("Car entity must have View3d relationship target component");
        assert_eq!(
            view_target.len(),
            1,
            "Car must produce exactly one View3d proxy, got {}",
            view_target.len()
        );
        let proxy_entity = view_target[0];

        // Verify world-wide proxy count is exactly one
        let mut proxy_query = app
            .world_mut()
            .query::<(Entity, &View3dOf, &Transform, &Car3d)>();
        let all_proxies: Vec<_> = proxy_query.iter(app.world()).collect();
        assert_eq!(
            all_proxies.len(),
            1,
            "There must be exactly one 3D car proxy in the world"
        );
        assert_eq!(all_proxies[0].0, proxy_entity);
        assert_eq!(all_proxies[0].1.0, car_entity);

        // Verify proxy transform translation matches ground_to_3d(GroundPos)
        let proxy_transform = all_proxies[0].2;
        let expected_3d_pos = ground_to_3d(car_pos, half_height);
        assert_eq!(
            proxy_transform.translation, expected_3d_pos,
            "Proxy Transform.translation must match ground_to_3d(GroundPos)"
        );
        assert_eq!(
            proxy_transform.rotation,
            heading_to_3d(car_heading),
            "Proxy Transform.rotation must match heading_to_3d(Heading)"
        );

        // Movement update sync test: moving ground pos maintains match
        let new_pos = Vec2::new(200.0, 10.0);
        app.world_mut()
            .get_mut::<sim::GroundPos>(car_entity)
            .unwrap()
            .0 = new_pos;
        app.update();

        let updated_transform = *app.world().get::<Transform>(proxy_entity).unwrap();
        assert_eq!(
            updated_transform.translation,
            ground_to_3d(new_pos, half_height),
            "Updated proxy Transform must match ground_to_3d(GroundPos) after movement sync"
        );

        // Despawn car and verify proxy is automatically removed
        app.world_mut().despawn(car_entity);

        assert!(
            app.world().get_entity(car_entity).is_err(),
            "Car entity must be despawned"
        );
        assert!(
            app.world().get_entity(proxy_entity).is_err(),
            "Despawning car must remove its View3d proxy via linked_spawn"
        );

        let remaining_proxies = app
            .world_mut()
            .query_filtered::<Entity, With<Car3d>>()
            .iter(app.world())
            .count();
        assert_eq!(
            remaining_proxies, 0,
            "No 3D car proxies should remain after car despawn"
        );
    }
}
