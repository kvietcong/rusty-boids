use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, diagnostic::LogDiagnosticsPlugin, prelude::*};
use rand::prelude::*;
use std::collections::HashMap;

#[derive(Component, Clone, Debug, PartialEq)]
struct Direction(Vec2);

#[derive(Component, Clone, Debug, PartialEq, Eq)]
enum CreatureType {
    Boid,
    Chaser,
}

#[derive(Component, Clone, Debug, PartialEq)]
struct Speed(f32);

struct BoidFactors {
    color: Color,
    speed: f32,
    vision: f32,
    size: Vec2,
    cohesion: f32,
    separation: f32,
    alignment: f32,
    collision_avoidance: f32,
    scare: f32,
}

struct ChaserFactors {
    color: Color,
    speed: f32,
    vision: f32,
    size: Vec2,
    cohesion: f32,
    separation: f32,
    alignment: f32,
    collision_avoidance: f32,
    chase: f32,
}

const WIDTH: f32 = 1600.0;
const HEIGHT: f32 = 900.0;

const IS_MODULAR: bool = false; // true makes the program slower
const IS_DEBUGGING: bool = true;

const BOIDS: usize = 1000;
const CHASERS: usize = BOIDS / 100;

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_resolution(WIDTH, HEIGHT);
    window.set_title("Le Boids".to_string());
}

fn spawn_creature(
    rng: &mut ThreadRng,
    commands: &mut Commands,
    creature_type: CreatureType,
    boid_factors: &BoidFactors,
    chaser_factors: &ChaserFactors,
) {
    let x = rng.gen_range(-WIDTH / 2.0..WIDTH / 2.0);
    let y = rng.gen_range(-HEIGHT / 2.0..HEIGHT / 2.0);
    let direction = Direction(
        Vec2::new(rng.gen::<f32>() * 2.0 - 1.0, rng.gen::<f32>() * 2.0 - 1.0).normalize(),
    );
    let color: Color;
    let speed: Speed;
    let size: Vec2;
    let mut creature_commands = commands.spawn();

    if creature_type == CreatureType::Boid {
        color = boid_factors.color;
        size = boid_factors.size;
        speed = Speed(boid_factors.speed);
        creature_commands.insert(creature_type);
    } else if creature_type == CreatureType::Chaser {
        color = chaser_factors.color;
        size = chaser_factors.size;
        speed = Speed(chaser_factors.speed);
        creature_commands.insert(creature_type);
    } else {
        panic!("Unknown creature type: {:?}", creature_type);
    }

    creature_commands
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                color,
                custom_size: Some(size),
                ..Sprite::default()
            },
            transform: Transform {
                translation: Vec3::new(x, y, 0.0),
                rotation: Quat::from_rotation_z(direction.0.y.atan2(direction.0.x)),
                ..Transform::default()
            },
            ..SpriteBundle::default()
        })
        .insert(direction)
        .insert(speed);
}

fn add_boids(
    mut commands: Commands,
    boid_factors: Res<BoidFactors>,
    chaser_factors: Res<ChaserFactors>
) {
    let mut rng = rand::thread_rng();
    for _ in 1..=BOIDS {
        spawn_creature(
            &mut rng, &mut commands, CreatureType::Boid,
            boid_factors.as_ref(), chaser_factors.as_ref());
    }
    for _ in 1..=CHASERS {
        spawn_creature(
            &mut rng, &mut commands, CreatureType::Chaser,
            boid_factors.as_ref(), chaser_factors.as_ref());
    }
}

fn move_creatures_system(
    mut query: Query<(&mut Transform, &Direction, &CreatureType)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>
) {
    for (mut transform, direction, c_type) in query.iter_mut() {
        let speed = match c_type {
            CreatureType::Boid => boid_factors.speed,
            CreatureType::Chaser => chaser_factors.speed,
        };
        transform.translation.x += direction.0.x * speed * timer.delta_seconds();
        transform.translation.y += direction.0.y * speed * timer.delta_seconds();
        transform.rotation = Quat::from_rotation_z(direction.0.y.atan2(direction.0.x));
    }
}

fn border_system(mut query: Query<&mut Transform>, windows: ResMut<Windows>) {
    let window = windows.get_primary().unwrap();
    let width = window.width();
    let height = window.height();
    for mut transform in query.iter_mut() {
        if transform.translation.x > width / 2.0 {
            transform.translation.x = -width / 2.0;
        } else if transform.translation.x < -width / 2.0 {
            transform.translation.x = width / 2.0;
        }
        if transform.translation.y > height / 2.0 {
            transform.translation.y = -height / 2.0;
        } else if transform.translation.y < -height / 2.0 {
            transform.translation.y = height / 2.0;
        }
    }
}

fn scare_system(
    mut query: Query<(&mut Direction, &Transform, &CreatureType)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    let mut combos = query.iter_combinations_mut();
    while let Some([(mut dir_a, trans_a, type_a), (mut dir_b, trans_b, type_b)]) =
        combos.fetch_next()
    {
        if type_a == type_b {
            continue;
        }
        if *type_a == CreatureType::Boid {
            let distance = trans_a.translation.distance(trans_b.translation);
            let vision = match type_a {
                CreatureType::Boid => boid_factors.vision,
                CreatureType::Chaser => chaser_factors.vision,
            };
            if distance < vision {
                let run_direction = Vec2::new(
                    trans_a.translation.x - trans_b.translation.x,
                    trans_a.translation.y - trans_b.translation.y,
                )
                .normalize();
                dir_a.0 = dir_a
                    .0
                    .lerp(run_direction, boid_factors.scare * timer.delta_seconds());
            }
        }
        if *type_b == CreatureType::Boid {
            let distance = trans_b.translation.distance(trans_a.translation);
            let vision = match type_b {
                CreatureType::Boid => boid_factors.vision,
                CreatureType::Chaser => chaser_factors.vision,
            };
            if distance < vision {
                let run_direction = Vec2::new(
                    trans_b.translation.x - trans_a.translation.x,
                    trans_b.translation.y - trans_a.translation.y,
                )
                .normalize();
                dir_b.0 = dir_b
                    .0
                    .lerp(run_direction, boid_factors.scare * timer.delta_seconds());
            }
        }
    }
}

fn chase_system(
    mut query: Query<(Entity, &mut Direction, &Transform, &CreatureType)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    let mut targets = HashMap::new();
    for (id_a, _, trans_a, type_a) in query.iter() {
        for (id_b, _, trans_b, type_b) in query.iter() {
            if id_a == id_b {
                continue;
            }
            if *type_a == CreatureType::Chaser && *type_b == CreatureType::Boid {
                let distance = trans_a.translation.distance(trans_b.translation);
                let vision = match type_a {
                    CreatureType::Boid => boid_factors.vision,
                    CreatureType::Chaser => chaser_factors.vision,
                };
                if distance < vision {
                    if let Some((old_distance, _)) = targets.get(&id_a) {
                        if distance < *old_distance {
                            let chase_direction = Vec2::new(
                                trans_b.translation.x - trans_a.translation.x,
                                trans_b.translation.y - trans_a.translation.y,
                            )
                            .normalize();
                            targets.insert(id_a, (distance, Some(chase_direction)));
                        }
                    } else {
                        targets.insert(id_a, (distance, None));
                    }
                }
            }
        }
    }

    for (id, (_, some_chase_direction)) in targets {
        if let Some(chase_direction) = some_chase_direction {
            if let Ok((_, mut direction, _, _)) = query.get_mut(id) {
                direction.0 = direction
                    .0
                    .lerp(chase_direction, chaser_factors.chase * timer.delta_seconds());
            }
        }
    }
}

fn separation_system(
    mut query: Query<(Entity, &mut Direction, &Transform, &CreatureType)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    let mut changes = vec![];

    for (id_a, _dir_a, trans_a, type_a) in query.iter() {
        let mut average_position = Vec2::ZERO;
        let mut count = 0;

        for (id_b, _dir_b, trans_b, _type_b) in query.iter() {
            if id_a == id_b {
                continue;
            }
            let vision = match type_a {
                CreatureType::Boid => boid_factors.vision,
                CreatureType::Chaser => chaser_factors.vision,
            };
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < vision / 2.0 {
                average_position += Vec2::new(trans_b.translation.x, trans_b.translation.y);
                count += 1;
            }
        }

        if count > 0 {
            average_position /= count as f32;
            let away_direction = Vec2::new(
                trans_a.translation.x - average_position.x,
                trans_a.translation.y - average_position.y,
            )
            .normalize();
            changes.push((id_a, away_direction));
        }
    }

    for (id, away_direction) in changes {
        if let Ok((_, mut dir, _, c_type)) = query.get_mut(id) {
            let separation = match c_type {
                CreatureType::Boid => boid_factors.separation,
                CreatureType::Chaser => chaser_factors.separation,
            };
            dir.0 = dir
                .0
                .lerp(away_direction, separation * timer.delta_seconds())
                .normalize();
        }
    }
}

fn cohesion_system(
    mut query: Query<(Entity, &mut Direction, &Transform, &CreatureType)>,
    chase_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    let mut changes = vec![];

    for (id_a, _dir_a, trans_a, type_a) in query.iter() {
        let mut average_position = Vec2::ZERO;
        let mut count = 0;

        for (id_b, _dir_b, trans_b, _type_b) in query.iter() {
            if id_a == id_b {
                continue;
            }
            let vision = match type_a {
                CreatureType::Boid => boid_factors.vision,
                CreatureType::Chaser => chase_factors.vision,
            };
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < vision {
                average_position += Vec2::new(trans_b.translation.x, trans_b.translation.y);
                count += 1;
            }
        }

        if count > 0 {
            average_position /= count as f32;
            let center_direction = Vec2::new(
                average_position.x - trans_a.translation.x,
                average_position.y - trans_a.translation.y,
            )
            .normalize();
            changes.push((id_a, center_direction));
        }
    }

    for (id, center_direction) in changes {
        if let Ok((_, mut dir, _, c_type)) = query.get_mut(id) {
            let cohesion = match c_type {
                CreatureType::Boid => boid_factors.cohesion,
                CreatureType::Chaser => chase_factors.cohesion,
            };
            dir.0 = dir
                .0
                .lerp(center_direction, cohesion * timer.delta_seconds())
                .normalize();
        }
    }
}

fn alignment_system(
    mut query: Query<(Entity, &mut Direction, &Transform, &CreatureType)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    let mut changes = vec![];

    for (id_a, _dir_a, trans_a, type_a) in query.iter() {
        let mut average_direction = Vec2::ZERO;
        let mut count = 0;

        for (id_b, dir_b, trans_b, _type_b) in query.iter() {
            if id_a == id_b {
                continue;
            }
            let vision = match type_a {
                CreatureType::Boid => boid_factors.vision,
                CreatureType::Chaser => chaser_factors.vision,
            };
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < vision {
                average_direction += dir_b.0;
                count += 1;
            }
        }

        if count > 0 {
            average_direction /= count as f32;
            changes.push((id_a, average_direction));
        }
    }

    for (id, average_direction) in changes {
        if let Ok((_, mut dir, _, c_type)) = query.get_mut(id) {
            let alignment = match c_type {
                CreatureType::Boid => boid_factors.alignment,
                CreatureType::Chaser => chaser_factors.alignment,
            };
            dir.0 = dir
                .0
                .lerp(average_direction, alignment * timer.delta_seconds())
                .normalize();
        }
    }
}

fn collision_avoidance_system(
    mut query: Query<(&mut Direction, &Transform, &CreatureType)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    let mut combos = query.iter_combinations_mut();
    while let Some([(mut dir_a, trans_a, type_a), (mut dir_b, trans_b, type_b)]) = combos.fetch_next() {
        let distance = trans_a.translation.distance(trans_b.translation);
        let direction = Vec2::new(
            trans_a.translation.x - trans_b.translation.x,
            trans_a.translation.y - trans_b.translation.y,
        );
        let size = chaser_factors.size.x.max(boid_factors.size.x);
        if distance < size {
            dir_a.0 = dir_a.0.lerp(
                direction,
                if *type_a == CreatureType::Boid { boid_factors.collision_avoidance } else { chaser_factors.collision_avoidance } * timer.delta_seconds(),
            );
            dir_b.0 = dir_b.0.lerp(
                -direction,
                if *type_b == CreatureType::Boid { boid_factors.collision_avoidance } else { chaser_factors.collision_avoidance } * timer.delta_seconds(),
            );
        }
    }
}

// Does exhibit slightly different behavior than the modularized system.
fn all_in_one_system(
    mut query: Query<(Entity, &mut Direction, &Transform, &CreatureType)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    let mut targets = HashMap::new();
    let mut scare_forces = vec![];
    let mut cohesion_forces = vec![];
    let mut alignment_forces = vec![];
    let mut separation_forces = vec![];
    let mut collision_avoidance_forces = vec![];

    for (id_a, _dir_a, trans_a, type_a) in query.iter() {
        let mut average_position = Vec2::ZERO;
        let mut average_direction = Vec2::ZERO;
        let mut average_position_close = Vec2::ZERO;

        let mut in_vision = 0;
        let mut in_half_vision = 0;

        for (id_b, dir_b, trans_b, type_b) in query.iter() {
            if id_a == id_b {
                continue;
            }
            let distance = trans_a.translation.distance(trans_b.translation);
            let size = match type_a {
                CreatureType::Boid => boid_factors.size,
                CreatureType::Chaser => chaser_factors.size,
            };
            let vision = match type_a {
                CreatureType::Boid => boid_factors.vision,
                CreatureType::Chaser => chaser_factors.vision,
            };
            if distance < size.x {
                let away_direction = Vec2::new(
                    trans_a.translation.x - trans_b.translation.x,
                    trans_a.translation.y - trans_b.translation.y,
                )
                .normalize();
                collision_avoidance_forces.push((id_a, away_direction));
            }
            if distance < vision {
                average_position += Vec2::new(trans_b.translation.x, trans_b.translation.y);
                average_direction += dir_b.0;
                // Chase System
                if *type_a == CreatureType::Chaser && *type_b == CreatureType::Boid {
                    if let Some((old_distance, _)) = targets.get(&id_a) {
                        if distance < *old_distance {
                            let chase_direction = Vec2::new(
                                trans_b.translation.x - trans_a.translation.x,
                                trans_b.translation.y - trans_a.translation.y,
                            )
                            .normalize();
                            targets.insert(id_a, (distance, Some(chase_direction)));
                        }
                    } else {
                        targets.insert(id_a, (distance, None));
                    }
                }
                // Scare System
                if *type_a == CreatureType::Boid && *type_b == CreatureType::Chaser {
                    if distance < vision {
                        let run_direction = Vec2::new(
                            trans_a.translation.x - trans_b.translation.x,
                            trans_a.translation.y - trans_b.translation.y,
                        )
                        .normalize();
                        scare_forces.push((id_a, run_direction));
                    }
                }
                in_vision += 1;
            }
            if distance < vision / 2.0 {
                average_position_close += Vec2::new(trans_b.translation.x, trans_b.translation.y);
                in_half_vision += 1;
            }
        }

        if in_half_vision > 0 {
            average_position_close /= in_half_vision as f32;
            let away_direction = Vec2::new(
                trans_a.translation.x - average_position_close.x,
                trans_a.translation.y - average_position_close.y,
            )
            .normalize();
            separation_forces.push((id_a, away_direction));
        }
        if in_vision > 0 {
            average_position /= in_vision as f32;
            average_direction /= in_vision as f32;
            let center_direction = Vec2::new(
                average_position.x - trans_a.translation.x,
                average_position.y - trans_a.translation.y,
            )
            .normalize();
            cohesion_forces.push((id_a, center_direction));
            alignment_forces.push((id_a, average_direction));
        }
    }

    for (id, center_direction) in cohesion_forces {
        if let Ok((_, mut dir, _, c_type)) = query.get_mut(id) {
            let cohesion = match c_type {
                CreatureType::Boid => boid_factors.cohesion,
                CreatureType::Chaser => chaser_factors.cohesion,
            };
            dir.0 = dir
                .0
                .lerp(center_direction, cohesion * timer.delta_seconds())
                .normalize();
        }
    }

    for (id, away_direction) in separation_forces {
        if let Ok((_, mut dir, _, c_type)) = query.get_mut(id) {
            let separation = match c_type {
                CreatureType::Boid => boid_factors.separation,
                CreatureType::Chaser => chaser_factors.separation,
            };
            dir.0 = dir
                .0
                .lerp(away_direction, separation * timer.delta_seconds())
                .normalize();
        }
    }

    for (id, average_direction) in alignment_forces {
        if let Ok((_, mut dir, _, c_type)) = query.get_mut(id) {
            let alignment = match c_type {
                CreatureType::Boid => boid_factors.alignment,
                CreatureType::Chaser => chaser_factors.alignment,
            };
            dir.0 = dir
                .0
                .lerp(average_direction, alignment * timer.delta_seconds())
                .normalize();
        }
    }

    for (id, run_direction) in scare_forces {
        if let Ok((_, mut dir, _, _)) = query.get_mut(id) {
            dir.0 = dir
                .0
                .lerp(run_direction, boid_factors.scare * timer.delta_seconds())
                .normalize();
        }
    }

    for (id, away_direction) in collision_avoidance_forces {
        if let Ok((_, mut dir, _, c_type)) = query.get_mut(id) {
            let collision_avoidance = match c_type {
                CreatureType::Boid => boid_factors.collision_avoidance,
                CreatureType::Chaser => chaser_factors.collision_avoidance,
            };
            dir.0 = dir
                .0
                .lerp(
                    away_direction,
                    collision_avoidance * timer.delta_seconds(),
                )
                .normalize();
        }
    }

    for (id, (_, some_chase_direction)) in targets {
        if let Some(chase_direction) = some_chase_direction {
            if let Ok((_, mut direction, _, _)) = query.get_mut(id) {
                direction.0 = direction
                    .0
                    .lerp(chase_direction, chaser_factors.chase * timer.delta_seconds());
            }
        }
    }
}

fn main() {
    let boid_factors = BoidFactors {
        color: Color::WHITE,
        speed: 75.0,
        vision: 50.0,
        size: Vec2::new(10.0, 3.0),
        cohesion: 1.00,
        separation: 1.50,
        alignment: 3.00,
        collision_avoidance: 1.0,
        scare: 10.0,
    };

    let chaser_factors = ChaserFactors {
        color: Color::RED,
        speed: 70.0,
        vision: 70.0,
        size: Vec2::new(16.0, 4.0),
        cohesion: 1.00,
        separation: 1.50,
        alignment: 3.00,
        collision_avoidance: 1.0,
        chase: 5.0,
    };

    let mut app = App::new();

    // Resources and plugins
    app.add_plugins(DefaultPlugins)
        .insert_resource(boid_factors)
        .insert_resource(chaser_factors)
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(WindowDescriptor {
            title: "Bevy - Le Boids".to_string(),
            width: WIDTH,
            height: HEIGHT,
            ..WindowDescriptor::default()
        }); // For some reason, this doesn't do anything

    // Startup Things
    app.add_startup_system(add_boids)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_window); // IDK Why the window doesn't resize with the descriptor

    // Systems
    app.add_system(move_creatures_system)
        .add_system(border_system);
    if IS_MODULAR {
        // Creature System Split into parts
        app.add_system(alignment_system)
            .add_system(cohesion_system)
            .add_system(separation_system)
            .add_system(scare_system)
            .add_system(chase_system)
            .add_system(collision_avoidance_system);
    } else {
        // Creature System Compacted (Better Performance)
        app.add_system(all_in_one_system);
    } // The two systems behave slightly differently

    if IS_DEBUGGING {
        app.add_plugin(LogDiagnosticsPlugin::default())
            .add_plugin(FrameTimeDiagnosticsPlugin::default());
    }

    app.run();
}

// TODO:
// - Fix major differences in collision avoidance between all in one and modules
// - Make live-adjustable factors
