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

use crate::ui::UiPlugin;

#[derive(Default, Resource)]
pub struct Cursor {
    pub position: Vec2,
    pub button_states: [bool; 3],
}

#[derive(Component)]
struct MainCamera;

pub const IS_WEB: bool = cfg!(target_arch = "wasm32");

const WIDTH: u32 = if IS_WEB { 1300 } else { 1600 };
const HEIGHT: u32 = if IS_WEB { 600 } else { 900 };

fn setup_cameras(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn setup_window(mut primary_query: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = primary_query.single_mut() else {
        return;
    };
    window.resolution = WindowResolution::new(WIDTH, HEIGHT);
    window.title = "Le Boids".to_string();
}

fn cursor_system(
    mut cursor: ResMut<Cursor>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
    mut mouse_button_messages: MessageReader<MouseButtonInput>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Ok(window) = primary_query.single() else {
        return;
    };
    if let Some(screen_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, screen_pos) {
            cursor.position = world_pos;
        }
    }
    for message in mouse_button_messages.read() {
        let button_index = match message.button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => continue,
        };
        cursor.button_states[button_index] = message.state.is_pressed();
    }
}

fn main() {
    let mut app = App::new();

    // Startup Things
    app.insert_resource(Cursor::default())
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(WIDTH, HEIGHT),
                title: "Bevy - Le Boids".to_string(),
                resizable: true,
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }));

    app.add_systems(Startup, (setup_window, setup_cameras));
    app.add_systems(Update, cursor_system);

    // Diagnostic stuff (FPS printing, etc)
    app.add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(2),
            ..Default::default()
        },
    ));

    app.add_plugins((BoidsPlugin::default(), UiPlugin::default()));

    app.run();
}
