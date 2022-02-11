use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, diagnostic::LogDiagnosticsPlugin, prelude::*};
use rand::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
struct Direction(Vec2);

#[derive(Component, Clone, Debug)]
struct Boid(String);

#[derive(Component, Clone, Copy, Debug)]
struct Speed(f32);

#[derive(Component, Clone, Copy, Debug)]
struct Vision(f32);

const WIDTH: f32 = 1600.0;
const HEIGHT: f32 = 900.0;

const IS_MODULAR: bool = false; // true makes the program slower
const IS_DEBUGGING: bool = false;

const BOIDS: usize = 1000;

const VISION_RANGE: f32 = 40.0;
const SPEED: f32 = 75.0;
const SIZE: f32 = 10.0;

const COHESION_FACTOR: f32 = 1.00;
const ALIGNMENT_FACTOR: f32 = 1.50;
const SEPARATION_FACTOR: f32 = 3.00;
const COLLISION_AVOIDANCE_FACTOR: f32 = 5.00;

fn add_boids(mut commands: Commands) {
    let mut rng = rand::thread_rng();
    for i in 1..=BOIDS {
        let x = rng.gen_range(-WIDTH / 2.0..WIDTH / 2.0);
        let y = rng.gen_range(-HEIGHT / 2.0..HEIGHT / 2.0);
        let direction = Direction(
            Vec2::new(rng.gen::<f32>() * 2.0 - 1.0, rng.gen::<f32>() * 2.0 - 1.0).normalize(),
        );
        let vision = Vision(VISION_RANGE);
        let speed = Speed(SPEED);
        commands
            .spawn()
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(SIZE, SIZE / 4.0)),
                    ..Sprite::default()
                },
                transform: Transform {
                    translation: Vec3::new(x, y, 0.0),
                    rotation: Quat::from_rotation_z(direction.0.y.atan2(direction.0.x)),
                    ..Transform::default()
                },
                ..SpriteBundle::default()
            })
            .insert(Boid(format!("Boid {}", i)))
            .insert(direction)
            .insert(vision)
            .insert(speed);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn move_boids_system(
    mut query: Query<(&mut Transform, &Direction, &Speed), With<Boid>>,
    timer: Res<Time>,
) {
    for (mut transform, direction, speed) in query.iter_mut() {
        transform.translation.x += direction.0.x * speed.0 * timer.delta_seconds();
        transform.translation.y += direction.0.y * speed.0 * timer.delta_seconds();
        transform.rotation = Quat::from_rotation_z(direction.0.y.atan2(direction.0.x));
    }
}

fn border_system(mut query: Query<&mut Transform, With<Boid>>, windows: ResMut<Windows>) {
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

fn separation_system(
    mut query: Query<(Entity, &mut Direction, &Vision, &Transform), With<Boid>>,
    timer: Res<Time>,
) {
    let mut changes = vec![];

    for boid_a in query.iter() {
        let mut average_position = Vec2::ZERO;
        let mut count = 0;
        let (id_a, _, vision, pos_a) = boid_a;

        for boid_b in query.iter() {
            let (id_b, _, _, pos_b) = boid_b;
            if id_a == id_b {
                continue;
            }
            let distance = pos_a.translation.distance(pos_b.translation);
            if distance < vision.0 / 2.0 {
                average_position += Vec2::new(pos_b.translation.x, pos_b.translation.y);
                count += 1;
            }
        }

        if count > 0 {
            average_position /= count as f32;
            let away_direction = Vec2::new(
                pos_a.translation.x - average_position.x,
                pos_a.translation.y - average_position.y,
            )
            .normalize();
            changes.push((id_a, away_direction));
        }
    }

    for (id, away_direction) in changes {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(away_direction, timer.delta_seconds() * SEPARATION_FACTOR)
                .normalize();
        }
    }
}

fn cohesion_system(
    mut query: Query<(Entity, &mut Direction, &Vision, &Transform), With<Boid>>,
    timer: Res<Time>,
) {
    let mut changes = vec![];

    for boid_a in query.iter() {
        let mut average_position = Vec2::ZERO;
        let mut count = 0;
        let (id_a, _, vision, pos_a) = boid_a;

        for boid_b in query.iter() {
            let (id_b, _, _, pos_b) = boid_b;
            if id_a == id_b {
                continue;
            }
            let distance = pos_a.translation.distance(pos_b.translation);
            if distance < vision.0 {
                average_position += Vec2::new(pos_b.translation.x, pos_b.translation.y);
                count += 1;
            }
        }

        if count > 0 {
            average_position /= count as f32;
            let center_direction = Vec2::new(
                average_position.x - pos_a.translation.x,
                average_position.y - pos_a.translation.y,
            )
            .normalize();
            changes.push((id_a, center_direction));
        }
    }

    for (id, center_direction) in changes {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(center_direction, timer.delta_seconds() * COHESION_FACTOR)
                .normalize();
        }
    }
}

fn alignment_system(
    mut query: Query<(Entity, &mut Direction, &Vision, &Transform), With<Boid>>,
    timer: Res<Time>,
) {
    let mut changes = vec![];

    for boid_a in query.iter() {
        let mut average_direction = Vec2::ZERO;
        let mut count = 0;
        let (id_a, _, vision, pos_a) = boid_a;

        for boid_b in query.iter() {
            let (id_b, direction_b, _, pos_b) = boid_b;
            if id_a == id_b {
                continue;
            }
            let distance = pos_a.translation.distance(pos_b.translation);
            if distance < vision.0 {
                average_direction += direction_b.0;
                count += 1;
            }
        }

        if count > 0 {
            average_direction /= count as f32;
            changes.push((id_a, average_direction));
        }
    }

    for (id, average_direction) in changes {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(average_direction, timer.delta_seconds() * ALIGNMENT_FACTOR)
                .normalize();
        }
    }
}

fn collision_avoidance_system(
    mut query: Query<(Entity, &mut Direction, &Transform), With<Boid>>,
    timer: Res<Time>,
) {
    let mut changes = vec![];

    for boid_a in query.iter() {
        let (id_a, _, pos_a) = boid_a;
        for boid_b in query.iter() {
            let (id_b, _, pos_b) = boid_b;
            if id_a == id_b {
                continue;
            }
            let distance = pos_a.translation.distance(pos_b.translation);
            if distance < SIZE {
                let away_direction = Vec2::new(
                    pos_a.translation.x - pos_b.translation.x,
                    pos_a.translation.y - pos_b.translation.y,
                )
                .normalize();
                changes.push((id_a, away_direction));
            }
        }
    }

    for (id, away_direction) in changes {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(
                    away_direction,
                    timer.delta_seconds() * COLLISION_AVOIDANCE_FACTOR,
                )
                .normalize();
        }
    }
}

fn all_in_on_system(
    mut query: Query<(Entity, &mut Direction, &Vision, &Transform), With<Boid>>,
    timer: Res<Time>,
) {
    let mut cohesion_forces = vec![];
    let mut separation_forces = vec![];
    let mut alignment_forces = vec![];
    let mut collision_avoidance_forces = vec![];

    for boid_a in query.iter() {
        let mut average_position = Vec2::ZERO;
        let mut average_position_close = Vec2::ZERO;
        let mut average_direction = Vec2::ZERO;
        let mut count = 0;
        let mut count_close = 0;

        let (id_a, _, vision, pos_a) = boid_a;

        for boid_b in query.iter() {
            let (id_b, dir_b, _, pos_b) = boid_b;
            if id_a == id_b {
                continue;
            }
            let distance = pos_a.translation.distance(pos_b.translation);
            if distance < SIZE {
                let away_direction = Vec2::new(
                    pos_a.translation.x - pos_b.translation.x,
                    pos_a.translation.y - pos_b.translation.y,
                )
                .normalize();
                collision_avoidance_forces.push((id_a, away_direction));
            }
            if distance < vision.0 {
                average_position += Vec2::new(pos_b.translation.x, pos_b.translation.y);
                average_direction += dir_b.0;
                count += 1;
            }
            if distance < vision.0 / 2.0 {
                average_position_close += Vec2::new(pos_b.translation.x, pos_b.translation.y);
                count_close += 1;
            }
        }

        if count_close > 0 {
            average_position_close /= count_close as f32;
            let away_direction = Vec2::new(
                pos_a.translation.x - average_position_close.x,
                pos_a.translation.y - average_position_close.y,
            )
            .normalize();
            separation_forces.push((id_a, away_direction));
        }
        if count > 0 {
            average_position /= count as f32;
            average_direction /= count as f32;
            let center_direction = Vec2::new(
                average_position.x - pos_a.translation.x,
                average_position.y - pos_a.translation.y,
            )
            .normalize();
            cohesion_forces.push((id_a, center_direction));
            alignment_forces.push((id_a, average_direction));
        }
    }

    for (id, center_direction) in cohesion_forces {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(center_direction, timer.delta_seconds() * COHESION_FACTOR)
                .normalize();
        }
    }

    for (id, away_direction) in separation_forces {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(away_direction, timer.delta_seconds() * SEPARATION_FACTOR)
                .normalize();
        }
    }

    for (id, average_direction) in alignment_forces {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(average_direction, timer.delta_seconds() * ALIGNMENT_FACTOR)
                .normalize();
        }
    }

    for (id, away_direction) in collision_avoidance_forces {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(
                    away_direction,
                    timer.delta_seconds() * COLLISION_AVOIDANCE_FACTOR,
                )
                .normalize();
        }
    }
}

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_resolution(WIDTH, HEIGHT);
    window.set_title("Le Boids".to_string());
}

fn main() {
    let mut app = App::new();

    // Resources and plugins
    app.add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(WindowDescriptor {
            title: "Bevy - Movement".to_string(),
            width: WIDTH,
            height: HEIGHT,
            ..WindowDescriptor::default()
        }); // For some reason, this doesn't do anything

    // Startup Things
    app.add_startup_system(add_boids)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_window); // IDK Why the window doesn't resize with the descriptor

    // Systems
    app.add_system(move_boids_system).add_system(border_system);
    if IS_MODULAR {
        // Boid System Modules
        app.add_system(alignment_system)
            .add_system(cohesion_system)
            .add_system(separation_system)
            .add_system(collision_avoidance_system);
    } else {
        // Boid System Compacted (Better Performance)
        app.add_system(all_in_on_system);
    }

    if IS_DEBUGGING {
        // Debug Systems
        app.add_plugin(LogDiagnosticsPlugin::default())
            .add_plugin(FrameTimeDiagnosticsPlugin::default());
    }

    app.run();
}
