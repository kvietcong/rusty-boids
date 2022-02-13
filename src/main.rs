use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use std::time::Duration;
mod boids;
use boids::*;
mod ui;
use ui::*;

const WIDTH: f32 = 1600.0;
const HEIGHT: f32 = 900.0;

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_resolution(WIDTH, HEIGHT);
    window.set_title("Le Boids".to_string());
}

fn main() {
    let mut app = App::new();

    // Startup Things
    app.add_plugins(DefaultPlugins)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_window) // IDK Why the window doesn't resize with the descriptor
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(WindowDescriptor {
            title: "Bevy - Le Boids".to_string(),
            width: WIDTH,
            height: HEIGHT,
            ..WindowDescriptor::default()
        }); // For some reason, this doesn't do anything

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
