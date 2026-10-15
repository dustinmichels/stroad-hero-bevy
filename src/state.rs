use bevy::prelude::*;

#[derive(States, Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
pub enum AppState {
    #[default]
    InGame,
    GameOver,
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
}
