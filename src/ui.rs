use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    utils::HashMap,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};

use crate::{
    boids::{KillProperties, SpawnProperties},
    CreatureType, Factors,
};

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

fn settings_system(
    keys: Res<Input<KeyCode>>,
    mut windows: ResMut<Windows>,
    mut egui_context: ResMut<EguiContext>,
    mut kill_properties: ResMut<KillProperties>,
    mut spawn_properties: ResMut<SpawnProperties>,
) {
    egui::Window::new("Settings")
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .vscroll(true)
        .show(egui_context.ctx_mut(), |ui| {
            if cfg!(target_arch = "wasm32") {
                ui.label(concat!(
                    "LShift and LCtrl detection are a little buggy on the web. ",
                    "The sim can keep keys pressed when you click out. ",
                    "Just click Ctrl and Shift while focused on the sim to reset input."
                ));
            }
            ui.collapsing("Spawning (LShift+Click to Spawn)", |ui| {
                ui.add(egui::Slider::new(&mut spawn_properties.radius, 5.0..=500.0).text("Radius"));
                ui.add(
                    egui::Slider::new(&mut spawn_properties.amount, 0..=100)
                        .text("Amount Per Click"),
                );
                egui::CollapsingHeader::new(format!(
                    "Type {}",
                    spawn_properties.creature_type.to_string()
                ))
                .default_open(true)
                .show(ui, |ui| {
                    for creature_type in CreatureType::all() {
                        ui.radio_value(
                            &mut spawn_properties.creature_type,
                            creature_type,
                            creature_type.to_string(),
                        );
                    }
                });
            });

            ui.collapsing("Killing (LCtrl+Click to Kill)", |ui| {
                ui.add(egui::Slider::new(&mut kill_properties.radius, 5.0..=500.0).text("Radius"));
                egui::CollapsingHeader::new("Types Impacted")
                    .default_open(true)
                    .show(ui, |ui| {
                        for creature_type in CreatureType::all() {
                            let text;
                            let is_killed = kill_properties.creature_types.contains(&creature_type);
                            if is_killed {
                                text = format!("Will Kill {}", creature_type.to_string());
                            } else {
                                text = format!("Will Not Kill {}", creature_type.to_string());
                            }
                            if ui.button(text).clicked() {
                                if is_killed {
                                    kill_properties.creature_types.remove(&creature_type);
                                } else {
                                    kill_properties.creature_types.insert(creature_type);
                                }
                            }
                        }
                    });
            });

            let window = windows.get_primary_mut().unwrap();
            let is_shift = keys.pressed(KeyCode::LShift);
            let is_ctrl = keys.pressed(KeyCode::LControl);
            ui.collapsing("Screen", |ui| {
                ui.label(
                    "Click to Increase. LCtrl+Click to Decrease. LShift+<> to increase change.",
                );
                let change = if is_shift { 500 } else { 50 };
                let change = if is_ctrl { -change } else { change };
                let change = change as f32;
                if ui.button("Width").clicked() {
                    let new_width = (window.width() + change).max(500.0);
                    window.set_resolution(new_width, window.height());
                }
                if ui.button("Height").clicked() {
                    let new_height = (window.height() + change).max(500.0);
                    window.set_resolution(window.width(), new_height);
                }
            });
        });
}

fn factors_system(
    mut egui_context: ResMut<EguiContext>,
    mut all_factors: ResMut<HashMap<CreatureType, Factors>>,
) {
    egui::Window::new("Edit Factors")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
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
                            ui.radio_value(&mut factors.color, Color::rgb(0.2, 0.5, 1.0), "Blue");
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

        app.add_system(factors_system).add_system(settings_system);
    }
}
