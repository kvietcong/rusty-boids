use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::mouse::MouseButtonInput,
    prelude::*,
    window::{PrimaryWindow, WindowResolution},
};
mod boids;
mod ui;
use boids::*;
use std::time::Duration;

#[derive(Default, Resource)]
pub struct Cursor {
    pub position: Vec2,
    pub button_states: [bool; 3],
}

#[derive(Component)]
struct MainCamera;

pub const IS_WASM: bool = cfg!(target_arch = "wasm32");

// Got to find out why these `cfg` directives with `wasm` don't work for me
// Weird that the macro works though...
// #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const WIDTH: f32 = if IS_WASM { 1300.0 } else { 1600.0 };

// #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const HEIGHT: f32 = if IS_WASM { 600.0 } else { 900.0 };

// #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
// const HEIGHT: f32 = 1600.0;

// #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
// const HEIGHT: f32 = 600.0;

fn setup_cameras(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default()).insert(MainCamera);
}

fn setup_window(mut primary_query: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = primary_query.get_single_mut().unwrap();
    window.resolution = WindowResolution::new(WIDTH, HEIGHT);
    window.title = "Le Boids".to_string();
}

fn cursor_system(
    mut cursor: ResMut<Cursor>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let (camera, camera_transform) = camera_query.single();
    let window = primary_query.get_single().unwrap();
    if let Some(screen_pos) = window.cursor_position() {
        let window_size = Vec2::new(window.width(), window.height());

        let normalized_device_coordinates = (screen_pos / window_size) * 2.0 - Vec2::ONE;
        let normalized_device_coordinates_to_world =
            camera_transform.compute_matrix() * camera.projection_matrix().inverse();
        let world_pos = normalized_device_coordinates_to_world
            .project_point3(normalized_device_coordinates.extend(-1.0));
        let world_pos: Vec2 = world_pos.truncate();

        cursor.position = world_pos;
    }
    for event in mouse_button_events.iter() {
        let button_index = match event.button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => continue,
        };
        cursor.button_states[button_index] = event.state.is_pressed();
    }
}

fn main() {
    let mut app = App::new();

    // Startup Things
    app.add_startup_system(setup_window) // IDK Why the window doesn't resize with the descriptor
        .add_startup_system(setup_cameras)
        .insert_resource(Cursor::default())
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(WIDTH, HEIGHT),
                title: "Bevy - Le Boids".to_string(),
                resizable: true,
                ..default()
            }),
            ..default()
        }));

    app.add_system(cursor_system);

    // Adding Boids Simulation which includes the UI plugin
    app.add_plugin(BoidsPlugin::default());

    // Diagnostic stuff (FPS printing, etc)
    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(2),
            ..Default::default()
        });

    app.run();
}
