use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::{mouse::MouseButtonInput, ElementState},
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

#[derive(Default)]
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
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
    commands.spawn_bundle(UiCameraBundle::default());
}

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_resolution(WIDTH, HEIGHT);
    window.set_title("Le Boids".to_string());
}

fn handle_key_system(keys: Res<Input<KeyCode>>, mut debug_state: ResMut<State<DebugState>>) {
    if keys.just_pressed(KeyCode::D) && keys.just_pressed(KeyCode::LShift) {
        let current_sim_state = debug_state.current();
        let new_debug_state = match current_sim_state {
            DebugState::On => DebugState::Off,
            DebugState::Off => DebugState::On,
        };
        debug_state.set(new_debug_state).unwrap();
    }
}

fn cursor_system(
    windows: Res<Windows>,
    mut cursor: ResMut<Cursor>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let (camera, camera_transform) = camera_query.single();
    let window = windows.get(camera.window).unwrap();
    if let Some(screen_pos) = window.cursor_position() {
        let window_size = Vec2::new(window.width(), window.height());

        let normalized_device_coordinates = (screen_pos / window_size) * 2.0 - Vec2::ONE;
        let normalized_device_coordinates_to_world =
            camera_transform.compute_matrix() * camera.projection_matrix.inverse();
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
        cursor.button_states[button_index] = event.state == ElementState::Pressed;
    }
}

fn main() {
    let mut app = App::new();

    // Startup Things
    app.add_plugins(DefaultPlugins)
        .add_startup_system(setup_window) // IDK Why the window doesn't resize with the descriptor
        .add_startup_system(setup_cameras)
        .add_state(DebugState::Off)
        .insert_resource(Cursor::default())
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(WindowDescriptor {
            title: "Bevy - Le Boids".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: false,
            resizable: true,
            ..WindowDescriptor::default()
        }); // For some reason, this doesn't do anything

    app.add_system(handle_key_system).add_system(cursor_system);

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
