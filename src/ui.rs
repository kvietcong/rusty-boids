use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};

use crate::{BoidFactors, ChaserFactors};

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
    mut boid_factors: ResMut<BoidFactors>,
    mut chaser_factors: ResMut<ChaserFactors>,
) {
    egui::Window::new("Boid Factors")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-5.0, -5.0])
        .show(egui_context.ctx_mut(), |ui| {
            ui.add(egui::Slider::new(&mut boid_factors.speed, 20.0..=200.0).text("Speed"));
            ui.add(egui::Slider::new(&mut boid_factors.alignment, 0.0..=20.0).text("Alignment"));
            ui.add(egui::Slider::new(&mut boid_factors.cohesion, 0.0..=20.0).text("Cohesion"));
            ui.add(egui::Slider::new(&mut boid_factors.separation, 0.0..=20.0).text("Separation"));
            ui.add(
                egui::Slider::new(&mut boid_factors.collision_avoidance, 0.0..=20.0)
                    .text("Collision Avoidance"),
            );
            ui.add(egui::Slider::new(&mut boid_factors.scare, 0.0..=20.0).text("Scare"));
            ui.add(egui::Slider::new(&mut boid_factors.vision, 10.0..=200.0).text("Vision"));
        });

    egui::Window::new("Chaser Factors")
        .anchor(egui::Align2::LEFT_BOTTOM, [5.0, -5.0])
        .show(egui_context.ctx_mut(), |ui| {
            ui.add(egui::Slider::new(&mut chaser_factors.speed, 20.0..=200.0).text("Speed"));
            ui.add(egui::Slider::new(&mut chaser_factors.alignment, 0.0..=20.0).text("Alignment"));
            ui.add(egui::Slider::new(&mut chaser_factors.cohesion, 0.0..=20.0).text("Cohesion"));
            ui.add(
                egui::Slider::new(&mut chaser_factors.separation, 0.0..=20.0).text("Separation"),
            );
            ui.add(
                egui::Slider::new(&mut chaser_factors.collision_avoidance, 0.0..=20.0)
                    .text("Collision Avoidance"),
            );
            ui.add(egui::Slider::new(&mut chaser_factors.chase, 0.0..=20.0).text("Chase"));
            ui.add(egui::Slider::new(&mut chaser_factors.vision, 10.0..=200.0).text("Vision"));
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
