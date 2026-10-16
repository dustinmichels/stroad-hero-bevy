use bevy::prelude::*;

#[derive(States, Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
pub enum AppState {
    #[default]
    InGame,
    GameOver,
}

#[derive(States, Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
pub enum ViewMode {
    #[default]
    ThreeD,
    TopDown,
}

/// Top-level system sets defining explicit, deterministic frame execution order.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// User input, UI interactions, and game restart triggers
    Input,
    /// Core simulation: signal cycle, vehicle spawning, motion planning, movement
    Sim,
    /// Collision detection between entities and events
    Collision,
    /// Entity despawns and expired crash state cleanup
    Cleanup,
    /// View synchronization (3D models, camera tracking, HUD text)
    RenderSync,
    /// Gizmo debug drawing for active 2D view
    DebugRender,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_app_state_is_ingame() {
        assert_eq!(AppState::default(), AppState::InGame);
    }

    #[test]
    fn test_app_state_equality() {
        assert_ne!(AppState::InGame, AppState::GameOver);
    }

    #[test]
    fn test_initial_view_mode_is_threed() {
        assert_eq!(ViewMode::default(), ViewMode::ThreeD);
    }

    #[test]
    fn test_view_mode_equality() {
        assert_ne!(ViewMode::ThreeD, ViewMode::TopDown);
    }

    #[test]
    fn test_game_set_variants_equality() {
        assert_eq!(GameSet::Input, GameSet::Input);
        assert_ne!(GameSet::Input, GameSet::Sim);
        assert_ne!(GameSet::Collision, GameSet::Cleanup);
        assert_ne!(GameSet::RenderSync, GameSet::DebugRender);
    }
}
