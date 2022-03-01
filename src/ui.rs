use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    utils::HashMap,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};

use crate::{CreatureType, Factors};

#[derive(Component)]
struct FPSText;

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

fn egui_system(
    mut egui_context: ResMut<EguiContext>,
    mut all_factors: ResMut<HashMap<CreatureType, Factors>>,
) {
    egui::Window::new("Edit Factors")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-5.0, -5.0])
        .vscroll(true)
        .show(egui_context.ctx_mut(), |ui| {
            for creature_type in CreatureType::all() {
                let factors = all_factors.get_mut(&creature_type).unwrap();
                ui.collapsing(
                    format!("Creature Type {} Factors", creature_type.to_string()),
                    |ui| {
                        ui.collapsing("Color", |ui| {
                            ui.radio_value(&mut factors.color, Color::RED, "Red");
                            ui.radio_value(&mut factors.color, Color::GREEN, "Green");
                            ui.radio_value(&mut factors.color, Color::BLUE, "Blue");
                            ui.radio_value(&mut factors.color, Color::WHITE, "White");
                            ui.radio_value(&mut factors.color, Color::GOLD, "Gold");
                        });
                        ui.add(egui::Slider::new(&mut factors.speed, 20.0..=200.0).text("Speed"));
                        ui.add(
                            egui::Slider::new(&mut factors.alignment, 0.0..=20.0).text("Alignment"),
                        );
                        ui.add(
                            egui::Slider::new(&mut factors.cohesion, 0.0..=20.0).text("Cohesion"),
                        );
                        ui.add(
                            egui::Slider::new(&mut factors.separation, 0.0..=20.0)
                                .text("Separation"),
                        );
                        ui.add(
                            egui::Slider::new(&mut factors.collision_avoidance, 0.0..=20.0)
                                .text("Collision Avoidance"),
                        );
                        ui.add(egui::Slider::new(&mut factors.scare, 0.0..=20.0).text("Scare"));
                        ui.add(egui::Slider::new(&mut factors.chase, 0.0..=20.0).text("Scare"));
                        ui.add(egui::Slider::new(&mut factors.vision, 10.0..=200.0).text("Vision"));
                        ui.collapsing("Scared Of", |ui| {
                            for other_creature_type in CreatureType::all() {
                                if creature_type == other_creature_type {
                                    continue;
                                }
                                let text;
                                let is_scared = factors.scared_of.contains(&other_creature_type);
                                if is_scared {
                                    text = format!("Scared of {}", other_creature_type.to_string());
                                } else {
                                    text = format!(
                                        "Not Scared of {}",
                                        other_creature_type.to_string()
                                    );
                                }
                                if ui.button(text).clicked() {
                                    if is_scared {
                                        factors.scared_of.remove(&other_creature_type);
                                    } else {
                                        factors.scared_of.insert(other_creature_type);
                                    }
                                }
                            }
                        });
                        ui.collapsing("Chasing", |ui| {
                            for other_creature_type in CreatureType::all() {
                                if creature_type == other_creature_type {
                                    continue;
                                }
                                let text;
                                let is_chasing = factors.will_chase.contains(&other_creature_type);
                                if is_chasing {
                                    text = format!("Chasing {}", other_creature_type.to_string());
                                } else {
                                    text =
                                        format!("Not Chasing {}", other_creature_type.to_string());
                                }
                                if ui.button(text).clicked() {
                                    if is_chasing {
                                        factors.will_chase.remove(&other_creature_type);
                                    } else {
                                        factors.will_chase.insert(other_creature_type);
                                    }
                                }
                            }
                        });
                    },
                );
            }
        });
}

#[derive(Default)]
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .add_startup_system(fps_text_setup)
            .add_system(fps_text_update_system);

        app.add_system(egui_system);
    }
}
