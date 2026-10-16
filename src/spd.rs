use bevy::prelude::*;

/// Game variable: Senseless Pedestrian Deaths (SPD)
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default, Deref, DerefMut)]
pub struct Spd(pub u32);

pub type SPD = Spd;

impl Spd {
    pub fn new(count: u32) -> Self {
        Self(count)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

/// Marker component for the SPD HUD text in the top right
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpdText;

/// System to spawn the SPD counter display in the top right of the screen
pub fn setup_spd_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("SPD: 0"),
        TextFont::from_font_size(24.0),
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            right: Val::Px(16.0),
            ..default()
        },
        SpdText,
    ));
}

/// System to update the SPD HUD text when the Spd resource changes
pub fn update_spd_ui(spd: Res<Spd>, mut query: Query<&mut Text, With<SpdText>>) {
    if spd.is_changed() {
        for mut text in &mut query {
            text.0 = format!("SPD: {}", spd.0);
        }
    }
}

/// Plugin configuring the SPD counter resource and HUD display
pub struct SpdPlugin;

impl Plugin for SpdPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Spd>()
            .add_systems(Startup, setup_spd_ui)
            .add_systems(Update, update_spd_ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spd_default_and_increment() {
        let mut spd = Spd::default();
        assert_eq!(spd.0, 0);

        spd.increment();
        assert_eq!(spd.0, 1);

        spd.increment();
        assert_eq!(spd.0, 2);

        spd.reset();
        assert_eq!(spd.0, 0);
    }

    #[test]
    fn test_spd_alias_spd() {
        let spd = SPD::new(5);
        assert_eq!(spd.0, 5);
    }

    #[test]
    fn test_spd_ui_spawn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SpdPlugin);
        app.update();

        assert!(app.world().contains_resource::<Spd>());
        let mut q = app.world_mut().query_filtered::<&Text, With<SpdText>>();
        let text = q.single(app.world()).unwrap();
        assert_eq!(text.0, "SPD: 0");
    }

    #[test]
    fn test_spd_ui_update_on_resource_change() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SpdPlugin);
        app.update();

        // Increment SPD
        {
            let mut spd = app.world_mut().resource_mut::<Spd>();
            spd.increment();
        }
        app.update();

        let mut q = app.world_mut().query_filtered::<&Text, With<SpdText>>();
        let text = q.single(app.world()).unwrap();
        assert_eq!(text.0, "SPD: 1");

        // Increment again
        {
            let mut spd = app.world_mut().resource_mut::<Spd>();
            spd.increment();
        }
        app.update();

        let mut q = app.world_mut().query_filtered::<&Text, With<SpdText>>();
        let text = q.single(app.world()).unwrap();
        assert_eq!(text.0, "SPD: 2");
    }
}
