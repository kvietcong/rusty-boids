use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

use crate::DebugState;

#[derive(Component)]
struct FPSText;

#[derive(Default, Debug)]
struct Cursor(f32, f32);

impl Cursor {
    fn new(x: f32, y: f32) -> Self {
        Self(x, y)
    }

    fn set(&mut self, mouse: Cursor) {
        self.0 = mouse.0;
        self.1 = mouse.1;
    }
}

impl From<Vec2> for Cursor {
    fn from(vec: Vec2) -> Self {
        Self(vec.x, vec.y)
    }
}

fn fps_text_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(10.0),
                    left: Val::Px(10.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "FPS: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "0".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 30.0,
                            color: Color::GREEN,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(FPSText);
}

fn fps_text_update_system(
    diagnostics: Res<Diagnostics>,
    mut query: Query<&mut Text, With<FPSText>>,
) {
    for mut text in query.iter_mut() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                text.sections[1].value = format!("{:.0}", average);
                text.sections[1].style.color = if average > 60.0 {
                    Color::GREEN
                } else if average > 30.0 {
                    Color::YELLOW
                } else {
                    Color::RED
                };
            }
        }
    }
}

fn cursor_update_system(windows: Res<Windows>, mut cursor: ResMut<Cursor>) {
    if let Some(window) = windows.get_primary() {
        if let Some(absolute_cursor_position) = window.cursor_position() {
            let window_dimensions = Vec2::new(window.width() as f32, window.height() as f32);
            let final_position = absolute_cursor_position - window_dimensions / 2.0;
            cursor.set(final_position.into());
        }
    }
}

fn debug_mouse_system(cursor: Res<Cursor>) {
    println!("{:?}", cursor.as_ref());
}

#[derive(Default)]
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(fps_text_setup)
            .insert_resource(Cursor::new(0.0, 0.0))
            .add_system(fps_text_update_system)
            .add_system(cursor_update_system);

        app.add_system_set(SystemSet::on_update(DebugState::On).with_system(debug_mouse_system));
    }
}
