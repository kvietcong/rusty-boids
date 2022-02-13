use bevy::prelude::*;

pub struct UiPlugin;

impl Default for UiPlugin {
    fn default() -> Self {
        Self {}
    }
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {}
}
