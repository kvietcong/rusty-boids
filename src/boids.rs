use bevy::{ecs::query::QueryEntityError, prelude::*};
use rand::prelude::*;

pub const BOIDS: usize = 1000;
pub const CHASERS: usize = BOIDS / 100;

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct BoidFactors {
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

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct ChaserFactors {
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

#[derive(Component, Clone, Debug, PartialEq)]
struct Direction(Vec2);

#[derive(Component, Clone, Debug, PartialEq)]
struct Boid;

#[derive(Component, Clone, Debug, PartialEq)]
struct Chaser;

fn spawn_creature(
    rng: &mut ThreadRng,
    commands: &mut Commands,
    creature_type: &str,
    boid_factors: &BoidFactors,
    chaser_factors: &ChaserFactors,
    screen_width: f32,
    screen_height: f32,
) {
    let x = rng.gen_range(-screen_width / 2.0..screen_width / 2.0);
    let y = rng.gen_range(-screen_height / 2.0..screen_height / 2.0);
    let direction = Direction(
        Vec2::new(rng.gen::<f32>() * 2.0 - 1.0, rng.gen::<f32>() * 2.0 - 1.0).normalize(),
    );
    let color: Color;
    let size: Vec2;
    let mut creature_commands = commands.spawn();

    if creature_type == "boid" {
        creature_commands.insert(Boid);
        color = boid_factors.color;
        size = boid_factors.size;
    } else if creature_type == "chaser" {
        creature_commands.insert(Chaser);
        color = chaser_factors.color;
        size = chaser_factors.size;
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
        .insert(direction);
}

fn setup_creatures(
    mut commands: Commands,
    boid_factors: Res<BoidFactors>,
    chaser_factors: Res<ChaserFactors>,
    windows: Res<Windows>,
) {
    let window = windows.get_primary().unwrap();
    let screen_width = window.width();
    let screen_height = window.height();

    let mut rng = rand::thread_rng();
    for _ in 1..=BOIDS {
        spawn_creature(
            &mut rng,
            &mut commands,
            "boid",
            boid_factors.as_ref(),
            chaser_factors.as_ref(),
            screen_width,
            screen_height,
        );
    }
    for _ in 1..=CHASERS {
        spawn_creature(
            &mut rng,
            &mut commands,
            "chaser",
            boid_factors.as_ref(),
            chaser_factors.as_ref(),
            screen_width,
            screen_height,
        );
    }
}

fn move_system(
    mut query: Query<(&mut Transform, &Direction, Option<&Boid>, Option<&Chaser>)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    for (mut transform, direction, boid_opt, chaser_opt) in query.iter_mut() {
        let speed = match (boid_opt, chaser_opt) {
            (Some(_), None) => boid_factors.speed,
            (None, Some(_)) => chaser_factors.speed,
            _ => 0.0,
        };
        transform.translation.x += direction.0.x * speed * timer.delta_seconds();
        transform.translation.y += direction.0.y * speed * timer.delta_seconds();
        transform.rotation = Quat::from_rotation_z(direction.0.y.atan2(direction.0.x));
    }
}

fn wrap_borders_system(mut query: Query<&mut Transform>, windows: ResMut<Windows>) {
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
    mut boids: Query<(&mut Direction, &Transform), With<Boid>>,
    chasers: Query<&Transform, With<Chaser>>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    for (mut direction, trans_a) in boids.iter_mut() {
        for trans_b in chasers.iter() {
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < boid_factors.vision {
                let run_direction = Vec2::new(
                    trans_a.translation.x - trans_b.translation.x,
                    trans_a.translation.y - trans_b.translation.y,
                )
                .normalize();
                direction.0 = direction
                    .0
                    .lerp(run_direction, boid_factors.scare * timer.delta_seconds());
            }
        }
    }
}

fn chase_system(
    mut chasers: Query<(&mut Direction, &Transform), With<Chaser>>,
    boids: Query<&Transform, With<Boid>>,
    chaser_factors: Res<ChaserFactors>,
    timer: Res<Time>,
) {
    for (mut direction, trans_a) in chasers.iter_mut() {
        let mut closest_target = (0.0, None);
        for trans_b in boids.iter() {
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < chaser_factors.vision {
                closest_target = match closest_target {
                    (_, None) => (distance, Some(trans_b)),
                    (old_distance, Some(_)) => {
                        if old_distance > distance {
                            (distance, Some(trans_b))
                        } else {
                            closest_target
                        }
                    }
                };
            }
        }
        if let (_, Some(closest_trans)) = closest_target {
            let chase_direction = Vec2::new(
                closest_trans.translation.x - trans_a.translation.x,
                closest_trans.translation.y - trans_a.translation.y,
            )
            .normalize();
            direction.0 = direction.0.lerp(
                chase_direction,
                chaser_factors.chase * timer.delta_seconds(),
            );
        }
    }
}

fn apply_force(
    force: Vec2,
    factor: f32,
    delta_time: f32,
    creature: Result<(Entity, Mut<Direction>, &Transform, &Sprite), QueryEntityError>,
) {
    if let Ok((_, mut dir, _, _)) = creature {
        dir.0 = dir.0.lerp(force, factor * delta_time).normalize();
    }
}

fn boid_flocking_system(
    mut boid_query: Query<(Entity, &mut Direction, &Transform, &Sprite), With<Boid>>,
    boid_factors: Res<BoidFactors>,
    timer: Res<Time>,
) {
    let mut boids = vec![];
    boid_query.iter().for_each(|boid| boids.push(boid));
    let (cohesion_forces, alignment_forces, separation_forces, collision_avoidance_forces) =
        calculate_flocking_forces(boids, boid_factors.vision);
    let delta_time = timer.delta_seconds();
    for (id, force) in cohesion_forces {
        apply_force(
            force,
            boid_factors.cohesion,
            delta_time,
            boid_query.get_mut(id),
        );
    }
    for (id, force) in alignment_forces {
        apply_force(
            force,
            boid_factors.alignment,
            delta_time,
            boid_query.get_mut(id),
        );
    }
    for (id, force) in separation_forces {
        apply_force(
            force,
            boid_factors.separation,
            delta_time,
            boid_query.get_mut(id),
        );
    }
    for (id, force) in collision_avoidance_forces {
        apply_force(
            force,
            boid_factors.collision_avoidance,
            delta_time,
            boid_query.get_mut(id),
        );
    }
}

fn chaser_flocking_system(
    mut chaser_query: Query<(Entity, &mut Direction, &Transform, &Sprite), With<Chaser>>,
    chaser_factors: Res<ChaserFactors>,
    timer: Res<Time>,
) {
    let mut chasers = vec![];
    chaser_query.iter().for_each(|chaser| chasers.push(chaser));
    let (cohesion_forces, alignment_forces, separation_forces, collision_avoidance_forces) =
        calculate_flocking_forces(chasers, chaser_factors.vision);
    let delta_time = timer.delta_seconds();
    for (id, force) in cohesion_forces {
        apply_force(
            force,
            chaser_factors.cohesion,
            delta_time,
            chaser_query.get_mut(id),
        );
    }
    for (id, force) in alignment_forces {
        apply_force(
            force,
            chaser_factors.alignment,
            delta_time,
            chaser_query.get_mut(id),
        );
    }
    for (id, force) in separation_forces {
        apply_force(
            force,
            chaser_factors.separation,
            delta_time,
            chaser_query.get_mut(id),
        );
    }
    for (id, force) in collision_avoidance_forces {
        apply_force(
            force,
            chaser_factors.collision_avoidance,
            delta_time,
            chaser_query.get_mut(id),
        );
    }
}

fn calculate_flocking_forces(
    creatures: Vec<(Entity, &Direction, &Transform, &Sprite)>,
    vision: f32,
) -> (
    Vec<(Entity, Vec2)>,
    Vec<(Entity, Vec2)>,
    Vec<(Entity, Vec2)>,
    Vec<(Entity, Vec2)>,
) {
    let mut cohesion_forces = vec![];
    let mut alignment_forces = vec![];
    let mut separation_forces = vec![];
    let mut collision_avoidance_forces = vec![];

    for (id_a, _, trans_a, sprite_a) in creatures.iter() {
        let mut average_position = Vec2::ZERO;
        let mut average_direction = Vec2::ZERO;
        let mut average_close_position = Vec2::ZERO;
        let mut vision_count = 0;
        let mut half_vision_count = 0;

        for (id_b, dir_b, trans_b, _) in creatures.iter() {
            if id_a == id_b {
                continue;
            }
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < vision {
                vision_count += 1;
                average_position += Vec2::new(trans_b.translation.x, trans_b.translation.y);
                average_direction += dir_b.0;
            }
            if distance < vision / 2.0 {
                half_vision_count += 1;
                average_close_position += Vec2::new(trans_b.translation.x, trans_b.translation.y);
            }
            if let Some(size) = sprite_a.custom_size {
                if distance < size.x * 2.0 {
                    let away_direction = Vec2::new(
                        trans_a.translation.x - trans_b.translation.x,
                        trans_a.translation.y - trans_b.translation.y,
                    )
                    .normalize();
                    collision_avoidance_forces.push((*id_a, away_direction));
                }
            }
        }

        if vision_count > 0 {
            average_position /= vision_count as f32;
            average_direction /= vision_count as f32;
            let cohesion_force = Vec2::new(
                average_position.x - trans_a.translation.x,
                average_position.y - trans_a.translation.y,
            )
            .normalize();
            cohesion_forces.push((*id_a, cohesion_force));
            alignment_forces.push((*id_a, average_direction.normalize()));
        }
        if half_vision_count > 0 {
            average_close_position /= half_vision_count as f32;
            let separation_force = Vec2::new(
                trans_a.translation.x - average_close_position.x,
                trans_a.translation.y - average_close_position.y,
            )
            .normalize();
            separation_forces.push((*id_a, separation_force));
        }
    }

    return (
        cohesion_forces,
        alignment_forces,
        separation_forces,
        collision_avoidance_forces,
    );
}

fn update_factors_system(
    mut chaser_query: Query<&mut Sprite, (With<Chaser>, Without<Boid>)>,
    mut boid_query: Query<&mut Sprite, (With<Boid>, Without<Chaser>)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
) {
    if chaser_factors.is_changed() {
        for mut sprite in chaser_query.iter_mut() {
            sprite.color = chaser_factors.color;
            sprite.custom_size = Some(chaser_factors.size);
        }
    }
    if boid_factors.is_changed() {
        for mut sprite in boid_query.iter_mut() {
            sprite.color = boid_factors.color;
            sprite.custom_size = Some(boid_factors.size);
        }
    }
}

pub struct BoidsPlugin {
    initial_boid_factors: BoidFactors,
    initial_chaser_factors: ChaserFactors,
}

impl Default for BoidsPlugin {
    fn default() -> Self {
        Self {
            initial_boid_factors: BoidFactors {
                color: Color::WHITE,
                speed: 75.0,
                vision: 30.0,
                size: Vec2::new(10.0, 2.0),
                cohesion: 1.00,
                separation: 1.00,
                alignment: 3.00,
                collision_avoidance: 4.0,
                scare: 10.0,
            },
            initial_chaser_factors: ChaserFactors {
                color: Color::RED,
                speed: 70.0,
                vision: 50.0,
                size: Vec2::new(16.0, 4.0),
                cohesion: 3.00,
                separation: 1.50,
                alignment: 3.00,
                collision_avoidance: 3.0,
                chase: 5.0,
            },
        }
    }
}

impl Plugin for BoidsPlugin {
    fn build(&self, app: &mut App) {
        // Insert Resources
        app.insert_resource(self.initial_boid_factors)
            .insert_resource(self.initial_chaser_factors);

        // Start up
        app.add_startup_system(setup_creatures);

        app.add_system(move_system)
            .add_system(wrap_borders_system)
            .add_system(scare_system)
            .add_system(chase_system)
            .add_system(boid_flocking_system)
            .add_system(chaser_flocking_system)
            .add_system(update_factors_system);
    }
}
