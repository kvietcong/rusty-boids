use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    utils::HashMap,
};
use bevy_egui::{
    egui::{self, color_picker::color_edit_button_rgb, color::Hsva},
    EguiContext, EguiPlugin,
};

use crate::{
    boids::{KillProperties, SpawnProperties},
    CreatureType, Factors, IS_WASM,
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

fn statistics_system(
    creature_query: Query<&CreatureType>,
    mut egui_context: ResMut<EguiContext>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
) {
    let population_information = creature_query.iter().fold(
        vec![0; all_factors.len()],
        |mut population_information, &creature_type| {
            population_information[creature_type.0] += 1;
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
                            ui.label(format!("{}: {}", CreatureType(index), count));
                        });
                });
        });
}

fn settings_system(
    keys: Res<Input<KeyCode>>,
    mut windows: ResMut<Windows>,
    mut egui_context: ResMut<EguiContext>,
    selected_creature_type: Res<CreatureType>,
    mut kill_properties: ResMut<KillProperties>,
    mut spawn_properties: ResMut<SpawnProperties>,
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
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut creature_query: Query<(Entity, &mut CreatureType)>,
    mut selected_creature_type: ResMut<CreatureType>,
    mut all_factors: ResMut<HashMap<CreatureType, Factors>>,
) {
    egui::Window::new("Edit Factors")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .vscroll(true)
        .show(egui_context.ctx_mut(), |ui| {
            let mut selected_type_index = selected_creature_type.0;
            egui::ComboBox::from_label("Select a Creature Type")
                .selected_text(format!("{}", CreatureType(selected_type_index)))
                .show_ui(ui, |ui| {
                    (0..all_factors.len()).for_each(|creature_index| {
                        ui.horizontal(|ui| {
                            let factors = all_factors.get(&CreatureType(creature_index)).unwrap();
                            let color = Hsva::from_rgb([factors.color.r(), factors.color.g(), factors.color.b()]);
                            ui.selectable_value(
                                &mut selected_type_index,
                                creature_index,
                                CreatureType(creature_index).to_string(),
                            );
                            egui::widgets::color_picker::show_color(ui, color, egui::Vec2::new(10.0, 10.0));
                        });
                    });
                });
            selected_creature_type.0 = selected_type_index;

            if ui.button("Add New Creature Type").clicked() {
                let new_creature_type = CreatureType(all_factors.len());
                all_factors.insert(new_creature_type, Factors::default());
                selected_creature_type.0 = new_creature_type.0;
            }

            // This is so hacky. I hate this. I'm so sorry.
            if all_factors.len() > 1 && ui.button("Remove Current Creature Type").clicked() {
                let selected_index = selected_creature_type.0;
                for (entity, mut creature_type) in creature_query.iter_mut() {
                    if *creature_type.as_ref() == *selected_creature_type {
                        commands.entity(entity).despawn();
                    } else if creature_type.0 > selected_index {
                        creature_type.0 -= 1;
                    }
                }

                for (mut creature_type, mut factors) in all_factors.drain().collect::<Vec<_>>() {
                    if creature_type == *selected_creature_type {
                        continue;
                    } else if creature_type.0 > selected_index {
                        creature_type.0 -= 1;
                    }
                    factors.scared_of.remove(&selected_creature_type);
                    factors.will_chase.remove(&selected_creature_type);
                    for mut scared_of in factors.scared_of.drain().collect::<Vec<_>>() {
                        if scared_of == *selected_creature_type {
                            continue;
                        } else if scared_of.0 > selected_index {
                            scared_of.0 -= 1;
                        }
                        factors.scared_of.insert(scared_of);
                    }
                    for mut will_chase in factors.will_chase.drain().collect::<Vec<_>>() {
                        if will_chase == *selected_creature_type {
                            continue;
                        } else if will_chase.0 > selected_index {
                            will_chase.0 -= 1;
                        }
                        factors.will_chase.insert(will_chase);
                    }
                    all_factors.insert(creature_type, factors);
                }
                selected_creature_type.0 = selected_creature_type.0.min(all_factors.len() - 1);
            }

            ui.separator();

            let selected_creature_type = *selected_creature_type.as_ref();
            let all_creature_types = (0..all_factors.len())
                .map(|creature_index| CreatureType(creature_index))
                .collect::<Vec<_>>();
            let factors = all_factors.get_mut(&selected_creature_type).unwrap();

            ui.horizontal(|ui| {
                let mut color = [factors.color.r(), factors.color.g(), factors.color.b()];
                color_edit_button_rgb(ui, &mut color);
                factors.color = color.into();
                ui.label("Color");
            });

            ui.add(egui::Slider::new(&mut factors.speed, 20.0..=200.0).text("Speed"));
            ui.add(egui::Slider::new(&mut factors.alignment, 0.0..=20.0).text("Alignment"));
            ui.add(egui::Slider::new(&mut factors.cohesion, 0.0..=20.0).text("Cohesion"));
            ui.add(egui::Slider::new(&mut factors.separation, 0.0..=20.0).text("Separation"));
            ui.add(
                egui::Slider::new(&mut factors.collision_avoidance, 0.0..=20.0)
                    .text("Collision Avoidance"),
            );
            ui.add(egui::Slider::new(&mut factors.scare, 0.0..=20.0).text("Scare"));
            ui.add(egui::Slider::new(&mut factors.chase, 0.0..=20.0).text("Chase"));
            ui.add(egui::Slider::new(&mut factors.vision, 10.0..=200.0).text("Vision"));
            ui.add(egui::Slider::new(&mut factors.size.x, 0.5..=50.0).text("Width"));
            ui.add(egui::Slider::new(&mut factors.size.y, 0.5..=50.0).text("Length"));

            ui.collapsing("Scared of", |ui| {
                for &other_creature_type in all_creature_types.iter() {
                    if selected_creature_type == other_creature_type {
                        continue;
                    }
                    let mut is_scared = factors.scared_of.contains(&other_creature_type);
                    ui.checkbox(&mut is_scared, other_creature_type.to_string());
                    if is_scared {
                        factors.scared_of.insert(other_creature_type);
                    } else {
                        factors.scared_of.remove(&other_creature_type);
                    }
                }
            });

            ui.collapsing("Chasing", |ui| {
                for &other_creature_type in all_creature_types.iter() {
                    if selected_creature_type == other_creature_type {
                        continue;
                    }
                    let mut will_chase = factors.will_chase.contains(&other_creature_type);
                    ui.checkbox(&mut will_chase, other_creature_type.to_string());
                    if will_chase {
                        factors.will_chase.insert(other_creature_type);
                    } else {
                        factors.will_chase.remove(&other_creature_type);
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
            .add_startup_system(fps_text_setup);

        // Ensure I don't delete entities before they are compared
        // by adding the `after` clause. Despawn seems to take place on
        // the frame after calling. If not, then the Bevy scheduler
        // has the possibility of running the two systems simultaneously
        // which is a big no no when I have the chance to delete entities.
        app.add_system(factors_system.after("force_adding"));

        app.add_system(settings_system)
            .add_system(statistics_system)
            .add_system(fps_text_update_system);
    }
}
