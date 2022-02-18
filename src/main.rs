use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use std::time::Duration;
mod boids;
use boids::*;
mod ui;
use ui::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DebugState {
    On,
    Off,
}

const WIDTH: f32 = 1600.0;
const HEIGHT: f32 = 900.0;

fn setup_cameras(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
}

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_resolution(WIDTH, HEIGHT);
    window.set_title("Le Boids".to_string());
}

fn handle_input_system(keys: Res<Input<KeyCode>>, mut debug_state: ResMut<State<DebugState>>) {
    if keys.just_pressed(KeyCode::D) {
        let current_sim_state = debug_state.current();
        let new_debug_state = match current_sim_state {
            DebugState::On => DebugState::Off,
            DebugState::Off => DebugState::On,
        };
        debug_state.set(new_debug_state).unwrap();
    }
}

fn main() {
    let mut app = App::new();

    // Startup Things
    app.add_plugins(DefaultPlugins)
        .add_startup_system(setup_window) // IDK Why the window doesn't resize with the descriptor
        .add_startup_system(setup_cameras)
        .add_state(DebugState::Off)
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(WindowDescriptor {
            title: "Bevy - Le Boids".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: false,
            resizable: true,
            ..WindowDescriptor::default()
        }); // For some reason, this doesn't do anything

    app.add_system(handle_input_system);

    // Adding Boids Simulation
    app.add_plugin(BoidsPlugin::default());

    // Adding UI
    app.add_plugin(UiPlugin::default());

    // Diagnostic stuff (FPS printing, etc)
    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(2),
            ..Default::default()
        });

    app.run();
}

// TODO:
// - Make live-adjustable factors
// - Make flocking more streamlined
// - Have flocking groups
