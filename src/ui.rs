use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    utils::HashMap,
};
use bevy_egui::{
    egui::{self, color_picker::color_edit_button_rgb},
    EguiContext, EguiPlugin,
};

use crate::{
    boids::{KillProperties, SpawnProperties},
    CreatureType, Factors, IS_WASM,
};

#[derive(Component)]
struct FPSText;

#[derive(Default)]
pub struct SelectedCreatureType(pub CreatureType);

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

fn statistics_system(mut egui_context: ResMut<EguiContext>, creature_query: Query<&CreatureType>) {
    let population_information = creature_query.iter().fold(
        vec![0; CreatureType::all().len()], // TODO: Change to `variant_count` when that hits stable.
        |mut population_information, &creature_type| {
            population_information[creature_type as usize] += 1;
            population_information
        },
    );
    egui::Window::new("Statistics")
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .vscroll(true)
        .show(egui_context.ctx_mut(), |ui| {
            egui::CollapsingHeader::new("Populations")
                .default_open(true)
                .show(ui, |ui| {
                    population_information
                        .iter()
                        .enumerate()
                        .for_each(|(index, count)| {
                            ui.label(format!("{}: {}", CreatureType::from(index), count));
                        });
                });
        });
}

fn settings_system(
    keys: Res<Input<KeyCode>>,
    mut windows: ResMut<Windows>,
    mut egui_context: ResMut<EguiContext>,
    mut kill_properties: ResMut<KillProperties>,
    mut spawn_properties: ResMut<SpawnProperties>,
    selected_creature_type: Res<SelectedCreatureType>,
) {
    egui::Window::new("Settings")
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .vscroll(true)
        .show(egui_context.ctx_mut(), |ui| {
            if IS_WASM {
                ui.collapsing("⚠ Web Warning ⚠", |ui| {
                    ui.label(concat!(
                        "LShift and LCtrl detection are a little buggy on the web. ",
                        "The sim can keep keys pressed when you click out. ",
                        "Just click Ctrl and Shift while focused on the sim to reset input."
                    ));
                });
            }

            ui.collapsing(
                format!(
                    "Spawning Type {} (LShift+Click to Spawn)",
                    selected_creature_type.0
                ),
                |ui| {
                    ui.add(
                        egui::Slider::new(&mut spawn_properties.radius, 5.0..=500.0).text("Radius"),
                    );
                    ui.add(
                        egui::Slider::new(&mut spawn_properties.amount, 0..=100)
                            .text("Amount Per Click"),
                    );
                },
            );

            ui.collapsing(
                format!(
                    "Killing Type {} (LCtrl+Click to Kill)",
                    selected_creature_type.0
                ),
                |ui| {
                    ui.add(
                        egui::Slider::new(&mut kill_properties.radius, 5.0..=500.0).text("Radius"),
                    );
                },
            );

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
    mut selected_creature_type: ResMut<SelectedCreatureType>,
) {
    egui::Window::new("Edit Factors")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .vscroll(true)
        .show(egui_context.ctx_mut(), |ui| {
            let mut selected_type = selected_creature_type.0;
            egui::ComboBox::from_label("Select a Creature Type")
                .selected_text(format!("{}", selected_type))
                .show_ui(ui, |ui| {
                    for creature_type in CreatureType::all() {
                        ui.selectable_value(
                            &mut selected_type,
                            creature_type,
                            creature_type.to_str(),
                        );
                    }
                });

            selected_creature_type.0 = selected_type;
            let selected_creature_type = selected_creature_type.0;

            let factors = all_factors.get_mut(&selected_creature_type).unwrap();

            ui.label("Color");
            let mut color = [factors.color.r(), factors.color.g(), factors.color.b()];
            color_edit_button_rgb(ui, &mut color);
            factors.color = color.into();

            ui.add(egui::Slider::new(&mut factors.speed, 20.0..=200.0).text("Speed"));
            ui.add(egui::Slider::new(&mut factors.alignment, 0.0..=20.0).text("Alignment"));
            ui.add(egui::Slider::new(&mut factors.cohesion, 0.0..=20.0).text("Cohesion"));
            ui.add(egui::Slider::new(&mut factors.separation, 0.0..=20.0).text("Separation"));
            ui.add(
                egui::Slider::new(&mut factors.collision_avoidance, 0.0..=20.0)
                    .text("Collision Avoidance"),
            );
            ui.add(egui::Slider::new(&mut factors.scare, 0.0..=20.0).text("Scare"));
            ui.add(egui::Slider::new(&mut factors.chase, 0.0..=20.0).text("Scare"));
            ui.add(egui::Slider::new(&mut factors.vision, 10.0..=200.0).text("Vision"));
            ui.collapsing("Scared Of", |ui| {
                for other_creature_type in CreatureType::all() {
                    if selected_creature_type == other_creature_type {
                        continue;
                    }
                    let text;
                    let is_scared = factors.scared_of.contains(&other_creature_type);
                    if is_scared {
                        text = format!("Scared of {}", other_creature_type.to_str());
                    } else {
                        text = format!("Not Scared of {}", other_creature_type.to_str());
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
                    if selected_creature_type == other_creature_type {
                        continue;
                    }
                    let text;
                    let is_chasing = factors.will_chase.contains(&other_creature_type);
                    if is_chasing {
                        text = format!("Chasing {}", other_creature_type.to_str());
                    } else {
                        text = format!("Not Chasing {}", other_creature_type.to_str());
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
        });
}

#[derive(Default)]
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .insert_resource(SelectedCreatureType::default())
            .add_startup_system(fps_text_setup);

        app.add_system(factors_system)
            .add_system(settings_system)
            .add_system(statistics_system)
            .add_system(fps_text_update_system);
    }
}
