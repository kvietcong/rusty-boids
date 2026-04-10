use std::collections::HashMap;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{
    egui::{self, color_picker::color_edit_button_rgb, Rgba},
    EguiContextSettings, EguiContexts, EguiPlugin,
};

use bevy_egui::EguiPrimaryContextPass;

use crate::{
    boids::{DespawnProperties, Features, SpawnProperties},
    CreatureType, FactorInfo, Factors, IS_WEB,
};

#[derive(Component)]
struct FPSText;

fn fps_text_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..Default::default()
            },
            ZIndex(100),
            Visibility::Visible,
            Text::new(""),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 30.0,
                ..Default::default()
            },
            TextColor(Color::from(bevy::color::palettes::basic::WHITE)),
        ))
        .with_children(|parent| {
            parent.spawn((
                TextSpan::new("FPS: "),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    ..Default::default()
                },
                TextColor(Color::from(bevy::color::palettes::basic::WHITE)),
            ));
            parent.spawn((
                TextSpan::new("0"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    ..Default::default()
                },
                TextColor(Color::from(bevy::color::palettes::basic::GREEN)),
                FPSText,
            ));
        });
}

fn fps_text_update_system(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<(&mut TextSpan, &mut TextColor), With<FPSText>>,
) {
    for (mut span, mut color) in query.iter_mut() {
        if let Some(fps) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
        {
            span.0 = format!("{:>4.0}", fps);
            color.0 = if fps > 60.0 {
                Color::from(bevy::color::palettes::basic::GREEN)
            } else if fps > 30.0 {
                Color::from(bevy::color::palettes::basic::YELLOW)
            } else {
                Color::from(bevy::color::palettes::basic::RED)
            };
        }
    }
}

fn statistics_system(creature_query: Query<&CreatureType>, mut egui_context: EguiContexts) {
    let mut population_information = creature_query
        .iter()
        .fold(
            HashMap::<CreatureType, i64>::new(),
            |mut population_information, &creature_type| {
                *population_information.entry(creature_type).or_insert(0) += 1;
                population_information
            },
        )
        .into_iter()
        .collect::<Vec<_>>();
    population_information.sort_unstable();

    egui::Window::new("Statistics")
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .resizable(false)
        .show(
            egui_context.ctx_mut().expect("Egui context not found"),
            |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::CollapsingHeader::new("Populations")
                        .default_open(true)
                        .show(ui, |ui| {
                            population_information
                                .iter()
                                .for_each(|(creature_type, count)| {
                                    ui.label(format!("{}: {}", creature_type, count));
                                });
                        });
                });
            },
        );
}

fn settings_system(
    mut features: ResMut<Features>,
    mut egui_context: EguiContexts,
    selected_creature_type: Res<CreatureType>,
    mut spawn_properties: ResMut<SpawnProperties>,
    mut despawn_properties: ResMut<DespawnProperties>,
) {
    egui::Window::new("Settings")
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .resizable(false)
        .show(
            egui_context.ctx_mut().expect("Egui context not found"),
            |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if IS_WEB {
                        ui.collapsing("⚠ Web Warning ⚠", |ui| {
                            ui.label(concat!(
                            "Shift and Ctrl detection are a little buggy on the web. ",
                            "The sim can keep keys pressed when you click out. ",
                            "Just click Ctrl and Shift while focused on the sim to reset input."
                        ));
                        });
                    }

                    ui.collapsing(
                        format!(
                            "Spawning Type {} (Shift+Click to Spawn)",
                            selected_creature_type.0
                        ),
                        |ui| {
                            ui.add(
                                egui::Slider::new(&mut spawn_properties.radius, 5.0..=500.0)
                                    .text("Radius"),
                            );
                            ui.add(
                                egui::Slider::new(&mut spawn_properties.amount, 0..=200)
                                    .text("Amount Per Click"),
                            );
                        },
                    );

                    ui.collapsing(
                        format!(
                            "Despawn Type {} (Ctrl+Click to Despawn)",
                            selected_creature_type.0
                        ),
                        |ui| {
                            ui.add(
                                egui::Slider::new(&mut despawn_properties.radius, 5.0..=500.0)
                                    .text("Radius"),
                            );
                        },
                    );

                    ui.collapsing("Features", |ui| {
                        ui.label("Enable or Disable Simulation Features");
                        ui.checkbox(&mut features.chasing, "Chasing");
                        ui.checkbox(&mut features.running, "Running");
                        ui.checkbox(&mut features.flocking, "Flocking");
                        ui.checkbox(&mut features.killing, "Killing");
                        ui.checkbox(&mut features.energy_draining, "Energy Draining");
                    });
                });
            },
        );
}

fn factors_system(
    mut commands: Commands,
    mut egui_context: EguiContexts,
    mut all_factors: ResMut<FactorInfo>,
    mut selected_creature_type: ResMut<CreatureType>,
    mut creature_query: Query<(Entity, &mut CreatureType)>,
) {
    egui::Window::new("Edit Factors")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .resizable(false)
        .show(
            egui_context.ctx_mut().expect("Egui context not found"),
            |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut selected_type_index = selected_creature_type.0;
                    let mut creature_types = all_factors.factors.keys().collect::<Vec<_>>();
                    creature_types.sort_unstable();

                    egui::ComboBox::from_label("Select")
                        .selected_text(format!("{}", CreatureType(selected_type_index)))
                        .show_ui(ui, |ui| {
                            creature_types.iter().for_each(|creature_type| {
                                ui.horizontal(|ui| {
                                    let factors = all_factors.factors.get(creature_type).unwrap();
                                    let color_srgba = factors.color.to_srgba();
                                    let color = Rgba::from_rgb(
                                        color_srgba.red,
                                        color_srgba.green,
                                        color_srgba.blue,
                                    );
                                    ui.selectable_value(
                                        &mut selected_type_index,
                                        creature_type.0,
                                        creature_type.to_string(),
                                    );
                                    egui::widgets::color_picker::show_color(
                                        ui,
                                        color,
                                        egui::Vec2::new(10.0, 10.0),
                                    );
                                });
                            });
                        });

                    selected_creature_type.0 = selected_type_index;

                    ui.horizontal(|ui| {
                        if all_factors.factors.len() > 1 && ui.button("Remove Selected").clicked() {
                            let selected_index = selected_creature_type.0;
                            for (entity, mut creature_type) in creature_query.iter_mut() {
                                if *creature_type.as_ref() == *selected_creature_type {
                                    commands.entity(entity).despawn();
                                } else if creature_type.0 > selected_index {
                                    creature_type.0 -= 1;
                                }
                            }

                            for (mut creature_type, mut factors) in
                                all_factors.factors.drain().collect::<Vec<_>>()
                            {
                                if creature_type == *selected_creature_type {
                                    continue;
                                } else if creature_type.0 > selected_index {
                                    creature_type.0 -= 1;
                                }
                                factors.predator_of.remove(&selected_creature_type);
                                for mut prey in factors.predator_of.drain().collect::<Vec<_>>() {
                                    if prey == *selected_creature_type {
                                        continue;
                                    } else if prey.0 > selected_index {
                                        prey.0 -= 1;
                                    }
                                    factors.predator_of.insert(prey);
                                }
                                all_factors.factors.insert(creature_type, factors);
                            }
                            selected_creature_type.0 =
                                selected_creature_type.0.min(all_factors.factors.len() - 1);
                        }

                        if ui.button("Add New").clicked() {
                            let new_creature_type = CreatureType(all_factors.factors.len() + 1);
                            all_factors
                                .factors
                                .insert(new_creature_type, Factors::default());
                            selected_creature_type.0 = new_creature_type.0;
                        }
                    });

                    ui.separator();

                    let selected_creature_type = *selected_creature_type.as_ref();
                    let all_creature_types =
                        all_factors.factors.keys().copied().collect::<Vec<_>>();
                    let factors = all_factors
                        .factors
                        .get_mut(&selected_creature_type)
                        .unwrap();

                    ui.horizontal(|ui| {
                        let color_srgba = factors.color.to_srgba();
                        let mut color = [color_srgba.red, color_srgba.green, color_srgba.blue];
                        color_edit_button_rgb(ui, &mut color);
                        factors.color = Color::srgb(color[0], color[1], color[2]);
                        ui.label("Color");
                    });

                    ui.add(egui::Slider::new(&mut factors.speed, 5.0..=200.0).text("Speed"));
                    ui.add(egui::Slider::new(&mut factors.vision, 5.0..=100.0).text("Vision"));
                    ui.add(egui::Slider::new(&mut factors.size, 0.5..=10.0).text("Size"));
                    ui.add(
                        egui::Slider::new(&mut factors.max_energy, 20.0..=200.0).text("Max Energy"),
                    );

                    ui.collapsing("Boids System", |ui| {
                        ui.add(
                            egui::Slider::new(&mut factors.alignment, 0.0..=50.0).text("Alignment"),
                        );
                        ui.add(
                            egui::Slider::new(&mut factors.cohesion, 0.0..=50.0).text("Cohesion"),
                        );
                        ui.add(
                            egui::Slider::new(&mut factors.separation, 0.0..=50.0)
                                .text("Separation"),
                        );
                        ui.add(
                            egui::Slider::new(&mut factors.collision_avoidance, 0.0..=50.0)
                                .text("Collision Avoidance"),
                        );
                    });

                    ui.collapsing("Predator/Prey (Chase/Run) System", |ui| {
                        let factors = all_factors
                            .factors
                            .get_mut(&selected_creature_type)
                            .unwrap();

                        ui.label(concat!(
                            "You can be both predator and prey to another type. ",
                            "If both, running/chasing is determined by your run/chase factor. ",
                            "On contact and w/ killing, whomever has more energy shall prevail."
                        ));

                        ui.add(egui::Slider::new(&mut factors.chase, 0.0..=50.0).text("Chase"));
                        ui.add(egui::Slider::new(&mut factors.scare, 0.0..=50.0).text("Scare"));

                        ui.collapsing("Predator of", |ui| {
                            for &other_creature_type in all_creature_types.iter() {
                                if selected_creature_type == other_creature_type {
                                    continue;
                                }
                                let mut is_predator_of_other =
                                    factors.predator_of.contains(&other_creature_type);
                                ui.checkbox(
                                    &mut is_predator_of_other,
                                    other_creature_type.to_string(),
                                );
                                if is_predator_of_other {
                                    factors.predator_of.insert(other_creature_type.clone());
                                } else {
                                    factors.predator_of.remove(&other_creature_type);
                                }
                            }
                        });

                        ui.collapsing("Prey of", |ui| {
                            for &other_creature_type in all_creature_types.iter() {
                                let other_factors =
                                    all_factors.factors.get_mut(&other_creature_type).unwrap();
                                if selected_creature_type == other_creature_type {
                                    continue;
                                }
                                let mut is_scared_of_other =
                                    other_factors.predator_of.contains(&selected_creature_type);
                                ui.checkbox(
                                    &mut is_scared_of_other,
                                    other_creature_type.to_string(),
                                );
                                if is_scared_of_other {
                                    other_factors.predator_of.insert(selected_creature_type);
                                } else {
                                    other_factors.predator_of.remove(&selected_creature_type);
                                }
                            }
                        });
                    });
                });
            },
        );
}

fn update_ui_scale_factor(
    egui_context: Single<(&mut EguiContextSettings,)>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    if let Ok(window) = primary_window.single() {
        let (mut egui_settings,) = egui_context.into_inner();
        egui_settings.scale_factor = 2. / window.scale_factor();
    }
}

#[derive(Default)]
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_systems(Startup, fps_text_setup)
            .add_systems(Update, fps_text_update_system)
            .add_systems(
                EguiPrimaryContextPass,
                (
                    factors_system,
                    settings_system,
                    statistics_system,
                    update_ui_scale_factor,
                ),
            );
    }
}
