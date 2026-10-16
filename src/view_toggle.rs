use bevy::prelude::*;

use crate::state::{GameSet, ViewMode};
use crate::view_debug2d::TopDownCamera;
use crate::view3d::ThreeDCamera;
use bevy::ui::{IsDefaultUiCamera, UiTargetCamera};
use bevy::ui_widgets::Button;

pub const COLOR_TOGGLE_BTN_NORMAL: Color = Color::srgb(0.2, 0.2, 0.2);
pub const COLOR_TOGGLE_BTN_HOVERED: Color = Color::srgb(0.3, 0.3, 0.3);
pub const COLOR_TOGGLE_BTN_PRESSED: Color = Color::srgb(0.4, 0.4, 0.4);

/// Helper returning the alternative view mode label for the UI button.
pub fn alternative_view_mode_name(mode: ViewMode) -> &'static str {
    match mode {
        ViewMode::ThreeD => "2D Debug (V)",
        ViewMode::TopDown => "3D View (V)",
    }
}

/// Marker component for the view toggle UI button in the top left.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewToggleButton;

/// Marker component for the view toggle button text.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewToggleButtonText;

/// System to spawn the view toggle button in the top left.
pub fn setup_view_toggle_ui(mut commands: Commands) {
    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(16.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_TOGGLE_BTN_NORMAL),
            #[allow(deprecated)]
            Interaction::default(),
            ViewToggleButton,
        ))
        .with_child((
            Text::new(alternative_view_mode_name(ViewMode::ThreeD)),
            TextFont::from_font_size(16.0),
            TextColor(Color::srgb(0.95, 0.95, 0.95)),
            ViewToggleButtonText,
        ));
}

#[allow(deprecated)]
type ButtonInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static mut BackgroundColor),
    (Changed<Interaction>, With<ViewToggleButton>),
>;

/// System to toggle `ViewMode` when the UI button is pressed.
#[allow(deprecated)]
pub fn view_toggle_button_interaction(
    mut interaction_query: ButtonInteractionQuery,
    current_mode: Res<State<ViewMode>>,
    mut next_mode: ResMut<NextState<ViewMode>>,
) {
    for (interaction, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(COLOR_TOGGLE_BTN_PRESSED);
                let next = match current_mode.get() {
                    ViewMode::ThreeD => ViewMode::TopDown,
                    ViewMode::TopDown => ViewMode::ThreeD,
                };
                next_mode.set(next);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(COLOR_TOGGLE_BTN_HOVERED);
            }
            Interaction::None => {
                *bg_color = BackgroundColor(COLOR_TOGGLE_BTN_NORMAL);
            }
        }
    }
}

/// System to toggle `ViewMode` when the `V` key is pressed.
pub fn view_toggle_keyboard(
    keyboard_input: Option<Res<ButtonInput<KeyCode>>>,
    current_mode: Res<State<ViewMode>>,
    mut next_mode: ResMut<NextState<ViewMode>>,
) {
    let Some(keys) = keyboard_input else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyV) {
        let next = match current_mode.get() {
            ViewMode::ThreeD => ViewMode::TopDown,
            ViewMode::TopDown => ViewMode::ThreeD,
        };
        next_mode.set(next);
    }
}

/// System to update the view toggle button text when `ViewMode` changes.
pub fn update_view_toggle_button_text(
    view_mode: Res<State<ViewMode>>,
    mut text_query: Query<&mut Text, With<ViewToggleButtonText>>,
) {
    if view_mode.is_changed() {
        let label = alternative_view_mode_name(*view_mode.get());
        for mut text in &mut text_query {
            text.0 = label.to_string();
        }
    }
}
type ThreeDCamQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static mut Camera), (With<ThreeDCamera>, Without<TopDownCamera>)>;
type TopDownCamQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static mut Camera), (With<TopDownCamera>, Without<ThreeDCamera>)>;

/// System run on entering `ViewMode::ThreeD`: enables 3D camera and disables 2D debug camera,
/// transferring `IsDefaultUiCamera` to `ThreeDCamera` and updating any explicit `UiTargetCamera`
/// components on UI nodes so all UI/HUD remains attached and active.
pub fn enter_view_threed(
    mut commands: Commands,
    mut threed_cam: ThreeDCamQuery,
    mut topdown_cam: TopDownCamQuery,
    mut ui_targets: Query<&mut UiTargetCamera>,
) {
    let mut active_cam = None;
    for (entity, mut cam) in &mut threed_cam {
        cam.is_active = true;
        commands.entity(entity).insert(IsDefaultUiCamera);
        active_cam = Some(entity);
    }
    for (entity, mut cam) in &mut topdown_cam {
        cam.is_active = false;
        commands.entity(entity).remove::<IsDefaultUiCamera>();
    }
    if let Some(target) = active_cam {
        for mut ui_target in &mut ui_targets {
            ui_target.0 = target;
        }
    }
}

/// System run on entering `ViewMode::TopDown`: disables 3D camera and enables 2D debug camera,
/// transferring `IsDefaultUiCamera` to `TopDownCamera` and updating any explicit `UiTargetCamera`
/// components on UI nodes so all UI/HUD remains attached and active.
pub fn enter_view_topdown(
    mut commands: Commands,
    mut threed_cam: ThreeDCamQuery,
    mut topdown_cam: TopDownCamQuery,
    mut ui_targets: Query<&mut UiTargetCamera>,
) {
    let mut active_cam = None;
    for (entity, mut cam) in &mut threed_cam {
        cam.is_active = false;
        commands.entity(entity).remove::<IsDefaultUiCamera>();
    }
    for (entity, mut cam) in &mut topdown_cam {
        cam.is_active = true;
        commands.entity(entity).insert(IsDefaultUiCamera);
        active_cam = Some(entity);
    }
    if let Some(target) = active_cam {
        for mut ui_target in &mut ui_targets {
            ui_target.0 = target;
        }
    }
}
pub struct ViewTogglePlugin;

impl Plugin for ViewTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_view_toggle_ui)
            .add_systems(OnEnter(ViewMode::ThreeD), enter_view_threed)
            .add_systems(OnEnter(ViewMode::TopDown), enter_view_topdown)
            .add_systems(
                Update,
                (
                    (view_toggle_button_interaction, view_toggle_keyboard).in_set(GameSet::Input),
                    update_view_toggle_button_text
                        .in_set(GameSet::RenderSync)
                        .run_if(state_changed::<ViewMode>),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[test]
    fn test_enter_view_systems_direct() {
        let mut app = App::new();
        let cam_3d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: true,
                    ..default()
                },
                ThreeDCamera,
            ))
            .id();
        let cam_2d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: false,
                    ..default()
                },
                TopDownCamera,
            ))
            .id();

        // Run enter_view_topdown directly
        let mut schedule = Schedule::default();
        schedule.add_systems(enter_view_topdown);
        schedule.run(app.world_mut());
        schedule.apply_deferred(app.world_mut());

        assert!(!app.world().get::<Camera>(cam_3d).unwrap().is_active);
        assert!(app.world().get::<Camera>(cam_2d).unwrap().is_active);
        assert!(app.world().get::<IsDefaultUiCamera>(cam_3d).is_none());
        assert!(app.world().get::<IsDefaultUiCamera>(cam_2d).is_some());

        // Run enter_view_threed directly
        let mut schedule = Schedule::default();
        schedule.add_systems(enter_view_threed);
        schedule.run(app.world_mut());
        schedule.apply_deferred(app.world_mut());

        assert!(app.world().get::<Camera>(cam_3d).unwrap().is_active);
        assert!(!app.world().get::<Camera>(cam_2d).unwrap().is_active);
        assert!(app.world().get::<IsDefaultUiCamera>(cam_3d).is_some());
        assert!(app.world().get::<IsDefaultUiCamera>(cam_2d).is_none());
    }

    #[test]
    fn test_ui_camera_attachment_across_view_switches() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<ViewMode>()
            .add_plugins(ViewTogglePlugin);

        let cam_3d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: true,
                    ..default()
                },
                ThreeDCamera,
                IsDefaultUiCamera,
            ))
            .id();
        let cam_2d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: false,
                    ..default()
                },
                TopDownCamera,
            ))
            .id();

        app.update();

        // Initially ThreeDCamera has IsDefaultUiCamera
        assert!(app.world().get::<IsDefaultUiCamera>(cam_3d).is_some());
        assert!(app.world().get::<IsDefaultUiCamera>(cam_2d).is_none());

        // Switch to TopDown
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::TopDown);
        app.update();

        // TopDownCamera now has IsDefaultUiCamera, ThreeDCamera does not
        assert!(app.world().get::<IsDefaultUiCamera>(cam_3d).is_none());
        assert!(app.world().get::<IsDefaultUiCamera>(cam_2d).is_some());

        // Switch back to ThreeD
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::ThreeD);
        app.update();

        // ThreeDCamera now has IsDefaultUiCamera, TopDownCamera does not
        assert!(app.world().get::<IsDefaultUiCamera>(cam_3d).is_some());
        assert!(app.world().get::<IsDefaultUiCamera>(cam_2d).is_none());
    }

    #[test]
    fn test_on_enter_view_mode_state_transitions() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<ViewMode>()
            .add_plugins(ViewTogglePlugin);

        let cam_3d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: true,
                    ..default()
                },
                ThreeDCamera,
            ))
            .id();
        let cam_2d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: false,
                    ..default()
                },
                TopDownCamera,
            ))
            .id();

        app.update();

        // Switch to TopDown
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::TopDown);
        app.update();

        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::TopDown
        );
        assert!(!app.world().get::<Camera>(cam_3d).unwrap().is_active);
        assert!(app.world().get::<Camera>(cam_2d).unwrap().is_active);

        // Switch back to ThreeD
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::ThreeD);
        app.update();

        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );
        assert!(app.world().get::<Camera>(cam_3d).unwrap().is_active);
        assert!(!app.world().get::<Camera>(cam_2d).unwrap().is_active);
    }

    #[test]
    fn test_setup_view_toggle_ui_spawns_button_and_text() {
        let mut app = App::new();
        app.add_systems(Startup, setup_view_toggle_ui);
        app.update();

        let mut btn_q = app
            .world_mut()
            .query::<(&Button, &Node, &ViewToggleButton)>();
        let (_, node, _) = btn_q.iter(app.world()).next().expect("Button must exist");
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.top, Val::Px(16.0));
        assert_eq!(node.left, Val::Px(16.0));

        let mut text_q = app.world_mut().query::<(&Text, &ViewToggleButtonText)>();
        let (text, _) = text_q
            .iter(app.world())
            .next()
            .expect("Button text must exist");
        assert_eq!(text.0, "2D Debug (V)");
    }

    #[test]
    fn test_view_toggle_button_interaction_flips_mode_and_text() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<ViewMode>()
            .add_plugins(ViewTogglePlugin);
        app.update();

        // Verify initial state
        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );
        let btn_entity = app
            .world_mut()
            .query_filtered::<Entity, With<ViewToggleButton>>()
            .iter(app.world())
            .next()
            .unwrap();

        // Simulate pressing button
        #[allow(deprecated)]
        {
            let mut int = app.world_mut().get_mut::<Interaction>(btn_entity).unwrap();
            *int = Interaction::Pressed;
        }
        app.update();
        app.update();

        // ViewMode should have toggled to TopDown
        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::TopDown
        );
        let text_entity = app
            .world_mut()
            .query_filtered::<&Text, With<ViewToggleButtonText>>()
            .iter(app.world())
            .next()
            .unwrap();
        assert_eq!(text_entity.0, "3D View (V)");

        // Press button again
        #[allow(deprecated)]
        {
            let mut int = app.world_mut().get_mut::<Interaction>(btn_entity).unwrap();
            *int = Interaction::Pressed;
        }
        app.update();
        app.update();

        // ViewMode should have toggled back to ThreeD
        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );
        let text_entity = app
            .world_mut()
            .query_filtered::<&Text, With<ViewToggleButtonText>>()
            .iter(app.world())
            .next()
            .unwrap();
        assert_eq!(text_entity.0, "2D Debug (V)");
    }

    #[test]
    fn test_view_toggle_button_visual_feedback() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<ViewMode>()
            .add_plugins(ViewTogglePlugin);
        app.update();

        let btn_entity = app
            .world_mut()
            .query_filtered::<Entity, With<ViewToggleButton>>()
            .iter(app.world())
            .next()
            .unwrap();

        // Hovered
        #[allow(deprecated)]
        {
            let mut int = app.world_mut().get_mut::<Interaction>(btn_entity).unwrap();
            *int = Interaction::Hovered;
        }
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(btn_entity).unwrap().0,
            COLOR_TOGGLE_BTN_HOVERED
        );

        // None
        #[allow(deprecated)]
        {
            let mut int = app.world_mut().get_mut::<Interaction>(btn_entity).unwrap();
            *int = Interaction::None;
        }
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(btn_entity).unwrap().0,
            COLOR_TOGGLE_BTN_NORMAL
        );
    }

    #[test]
    fn test_view_toggle_keyboard_shortcut_v() {
        use bevy::input::ButtonInput;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<ViewMode>()
            .add_plugins(ViewTogglePlugin);
        app.update();

        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );

        // Pressing unrelated keys (W, A, S, D, ArrowUp, KeyR, Space) should not toggle ViewMode
        let non_toggle_keys = [
            KeyCode::KeyW,
            KeyCode::KeyA,
            KeyCode::KeyS,
            KeyCode::KeyD,
            KeyCode::ArrowUp,
            KeyCode::KeyR,
            KeyCode::Space,
        ];
        for key in non_toggle_keys {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            app.update();
            assert_eq!(
                *app.world().resource::<State<ViewMode>>().get(),
                ViewMode::ThreeD,
                "Key {:?} should not toggle ViewMode",
                key
            );
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(key);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear_just_pressed(key);
        }

        // Pressing V should toggle to TopDown
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyV);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::KeyV);
        app.update();
        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::TopDown
        );

        // Release V
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyV);
        app.update();

        // Pressing V again should toggle back to ThreeD
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyV);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::KeyV);
        app.update();
        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );
    }

    #[test]
    fn test_ui_target_camera_sync_across_view_switches() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<ViewMode>()
            .add_plugins(ViewTogglePlugin);

        let cam_3d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: true,
                    ..default()
                },
                ThreeDCamera,
                IsDefaultUiCamera,
            ))
            .id();
        let cam_2d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: false,
                    ..default()
                },
                TopDownCamera,
            ))
            .id();

        // Spawn a UI entity explicitly using UiTargetCamera
        let ui_entity = app
            .world_mut()
            .spawn((Node::default(), UiTargetCamera(cam_3d)))
            .id();

        app.update();
        assert_eq!(
            app.world().get::<UiTargetCamera>(ui_entity).unwrap().0,
            cam_3d
        );

        // Switch to TopDown
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::TopDown);
        app.update();
        assert_eq!(
            app.world().get::<UiTargetCamera>(ui_entity).unwrap().0,
            cam_2d
        );

        // Switch back to ThreeD
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::ThreeD);
        app.update();
        assert_eq!(
            app.world().get::<UiTargetCamera>(ui_entity).unwrap().0,
            cam_3d
        );
    }

    #[test]
    fn test_hud_elements_remain_active_across_view_switches() {
        use crate::spd::{self, Spd, SpdText};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<ViewMode>()
            .add_plugins(spd::SpdPlugin)
            .add_plugins(ViewTogglePlugin);

        let cam_3d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: true,
                    ..default()
                },
                ThreeDCamera,
                IsDefaultUiCamera,
            ))
            .id();
        let cam_2d = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: false,
                    ..default()
                },
                TopDownCamera,
            ))
            .id();

        app.update();

        // 1. Initial State in ViewMode::ThreeD
        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );
        assert!(app.world().get::<Camera>(cam_3d).unwrap().is_active);
        assert!(!app.world().get::<Camera>(cam_2d).unwrap().is_active);
        assert!(app.world().get::<IsDefaultUiCamera>(cam_3d).is_some());
        assert!(app.world().get::<IsDefaultUiCamera>(cam_2d).is_none());

        // Check SPD counter and ViewToggleButton exist
        let spd_text = app
            .world_mut()
            .query_filtered::<&Text, With<SpdText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(spd_text.0, "SPD: 0");

        let btn_text = app
            .world_mut()
            .query_filtered::<&Text, With<ViewToggleButtonText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(btn_text.0, "2D Debug (V)");

        // Increment SPD in ThreeD view
        app.world_mut().resource_mut::<Spd>().increment();
        app.update();
        let spd_text = app
            .world_mut()
            .query_filtered::<&Text, With<SpdText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(spd_text.0, "SPD: 1");

        // 2. Switch to ViewMode::TopDown
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::TopDown);
        app.update();

        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::TopDown
        );
        assert!(!app.world().get::<Camera>(cam_3d).unwrap().is_active);
        assert!(app.world().get::<Camera>(cam_2d).unwrap().is_active);
        assert!(app.world().get::<IsDefaultUiCamera>(cam_3d).is_none());
        assert!(app.world().get::<IsDefaultUiCamera>(cam_2d).is_some());

        // Button text should now display alternative view "3D View (V)"
        let btn_text = app
            .world_mut()
            .query_filtered::<&Text, With<ViewToggleButtonText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(btn_text.0, "3D View (V)");

        // Increment SPD in TopDown view - HUD must remain active and updating
        app.world_mut().resource_mut::<Spd>().increment();
        app.update();
        let spd_text = app
            .world_mut()
            .query_filtered::<&Text, With<SpdText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(spd_text.0, "SPD: 2");

        // 3. Switch back to ViewMode::ThreeD
        app.world_mut()
            .resource_mut::<NextState<ViewMode>>()
            .set(ViewMode::ThreeD);
        app.update();

        assert_eq!(
            *app.world().resource::<State<ViewMode>>().get(),
            ViewMode::ThreeD
        );
        assert!(app.world().get::<Camera>(cam_3d).unwrap().is_active);
        assert!(!app.world().get::<Camera>(cam_2d).unwrap().is_active);
        assert!(app.world().get::<IsDefaultUiCamera>(cam_3d).is_some());
        assert!(app.world().get::<IsDefaultUiCamera>(cam_2d).is_none());

        // Button text restored to "2D Debug (V)"
        let btn_text = app
            .world_mut()
            .query_filtered::<&Text, With<ViewToggleButtonText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(btn_text.0, "2D Debug (V)");

        // Increment SPD again in ThreeD view
        app.world_mut().resource_mut::<Spd>().increment();
        app.update();
        let spd_text = app
            .world_mut()
            .query_filtered::<&Text, With<SpdText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(spd_text.0, "SPD: 3");
    }

    #[test]
    fn test_toggle_flips_view_mode_exactly_one_camera_active_and_toggling_twice_restores_threed() {
        use bevy::input::ButtonInput;

        fn assert_single_active_camera(
            app: &mut App,
            expected_mode: ViewMode,
            cam_3d: Entity,
            cam_2d: Entity,
        ) {
            let mode = *app.world().resource::<State<ViewMode>>().get();
            assert_eq!(mode, expected_mode);

            let is_3d_active = app.world().get::<Camera>(cam_3d).unwrap().is_active;
            let is_2d_active = app.world().get::<Camera>(cam_2d).unwrap().is_active;

            match expected_mode {
                ViewMode::ThreeD => {
                    assert!(
                        is_3d_active,
                        "3D camera should be active in ViewMode::ThreeD"
                    );
                    assert!(
                        !is_2d_active,
                        "2D camera should be inactive in ViewMode::ThreeD"
                    );
                }
                ViewMode::TopDown => {
                    assert!(
                        !is_3d_active,
                        "3D camera should be inactive in ViewMode::TopDown"
                    );
                    assert!(
                        is_2d_active,
                        "2D camera should be active in ViewMode::TopDown"
                    );
                }
            }

            let mut cam_q = app.world_mut().query::<&Camera>();
            let active_count = cam_q.iter(app.world()).filter(|c| c.is_active).count();
            assert_eq!(
                active_count, 1,
                "Exactly one camera must be active across the world, found {}",
                active_count
            );
        }

        // === 1. Test button toggle flips ViewMode and toggling twice restores ThreeD ===
        {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, StatesPlugin))
                .init_state::<ViewMode>()
                .add_plugins(ViewTogglePlugin);

            let cam_3d = app
                .world_mut()
                .spawn((
                    Camera {
                        is_active: true,
                        ..default()
                    },
                    ThreeDCamera,
                    IsDefaultUiCamera,
                ))
                .id();
            let cam_2d = app
                .world_mut()
                .spawn((
                    Camera {
                        is_active: false,
                        ..default()
                    },
                    TopDownCamera,
                ))
                .id();

            app.update();

            // Initial state: ViewMode::ThreeD, exactly one camera active (3D)
            assert_single_active_camera(&mut app, ViewMode::ThreeD, cam_3d, cam_2d);

            let btn_entity = app
                .world_mut()
                .query_filtered::<Entity, With<ViewToggleButton>>()
                .iter(app.world())
                .next()
                .expect("ViewToggleButton must be spawned");

            // First toggle via button click: flips ViewMode to TopDown
            #[allow(deprecated)]
            {
                let mut int = app.world_mut().get_mut::<Interaction>(btn_entity).unwrap();
                *int = Interaction::Pressed;
            }
            app.update();
            app.update();

            assert_single_active_camera(&mut app, ViewMode::TopDown, cam_3d, cam_2d);

            // Second toggle via button click: restores ViewMode to ThreeD
            #[allow(deprecated)]
            {
                let mut int = app.world_mut().get_mut::<Interaction>(btn_entity).unwrap();
                *int = Interaction::Pressed;
            }
            app.update();
            app.update();

            assert_single_active_camera(&mut app, ViewMode::ThreeD, cam_3d, cam_2d);
        }

        // === 2. Test 'V' key shortcut flips ViewMode and toggling twice restores ThreeD ===
        {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, StatesPlugin))
                .init_resource::<ButtonInput<KeyCode>>()
                .init_state::<ViewMode>()
                .add_plugins(ViewTogglePlugin);

            let cam_3d = app
                .world_mut()
                .spawn((
                    Camera {
                        is_active: true,
                        ..default()
                    },
                    ThreeDCamera,
                    IsDefaultUiCamera,
                ))
                .id();
            let cam_2d = app
                .world_mut()
                .spawn((
                    Camera {
                        is_active: false,
                        ..default()
                    },
                    TopDownCamera,
                ))
                .id();

            app.update();

            // Initial state: ViewMode::ThreeD, exactly one camera active (3D)
            assert_single_active_camera(&mut app, ViewMode::ThreeD, cam_3d, cam_2d);

            // First toggle via 'V' key press: flips ViewMode to TopDown
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyV);
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear_just_pressed(KeyCode::KeyV);
            app.update();

            assert_single_active_camera(&mut app, ViewMode::TopDown, cam_3d, cam_2d);

            // Release 'V' key
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::KeyV);
            app.update();

            // Second toggle via 'V' key press: restores ViewMode to ThreeD
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyV);
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear_just_pressed(KeyCode::KeyV);
            app.update();

            assert_single_active_camera(&mut app, ViewMode::ThreeD, cam_3d, cam_2d);
        }
    }
}
