use bevy::prelude::*;
use rand::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
struct Direction(Vec2);

#[derive(Component, Clone, Debug)]
struct Boid(String);

#[derive(Component, Clone, Copy, Debug)]
struct Speed(f32);

#[derive(Component, Clone, Copy, Debug)]
struct Vision(f32);

const WIDTH: f32 = 1920.0;
const HEIGHT: f32 = 1080.0;

const VISION_RANGE: f32 = 75.0;
const SPEED: f32 = 150.0;

const COHESION_FACTOR: f32 = 1.50;
const ALIGNMENT_FACTOR: f32 = 1.00;
const SEPARATION_FACTOR: f32 = 2.50;

fn add_boids(mut commands: Commands) {
    let mut rng = rand::thread_rng();
    for i in 1..=100 {
        let x = rng.gen_range(-WIDTH / 2.0..WIDTH / 2.0);
        let y = rng.gen_range(-HEIGHT / 2.0..HEIGHT / 2.0);
        let direction = Direction(
            Vec2::new(rng.gen::<f32>() * 2.0 - 1.0, rng.gen::<f32>() * 2.0 - 1.0).normalize(),
        );
        let vision = Vision(VISION_RANGE);
        let speed = Speed(SPEED);
        commands
            .spawn()
            // The "draw" component
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(20.0, 5.0)),
                    ..Sprite::default()
                },
                transform: Transform {
                    translation: Vec3::new(x, y, 0.0),
                    rotation: Quat::from_rotation_z(direction.0.y.atan2(direction.0.x)),
                    ..Transform::default()
                },
                ..SpriteBundle::default()
            })
            // Tag to indicate Boid
            .insert(Boid(format!("Boid {}", i)))
            // Movement Direction
            .insert(direction)
            // Vision Radius
            .insert(vision)
            // Movement Speed
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
    let mut changes = Vec::new();

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
            let direction = Vec2::new(
                average_position.x - pos_a.translation.x,
                average_position.y - pos_a.translation.y,
            )
            .normalize()
                * -1.0;
            changes.push((id_a, direction));
        }
    }

    for (id, direction) in changes {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(direction, timer.delta_seconds() * SEPARATION_FACTOR)
                .normalize();
        }
    }
}

fn cohesion_system(
    mut query: Query<(Entity, &mut Direction, &Vision, &Transform), With<Boid>>,
    timer: Res<Time>,
) {
    let mut changes = Vec::new();

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
            let direction = Vec2::new(
                average_position.x - pos_a.translation.x,
                average_position.y - pos_a.translation.y,
            )
            .normalize();
            changes.push((id_a, direction));
        }
    }

    for (id, direction) in changes {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(direction, timer.delta_seconds() * COHESION_FACTOR)
                .normalize();
        }
    }
}

fn alignment_system(
    mut query: Query<(Entity, &mut Direction, &Vision, &Transform), With<Boid>>,
    timer: Res<Time>,
) {
    let mut changes = Vec::new();

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
            changes.push((id_a, average_direction.normalize()));
        }
    }

    for (id, direction) in changes {
        if let Ok(mut boid) = query.get_mut(id) {
            boid.1 .0 = boid
                .1
                 .0
                .lerp(direction, timer.delta_seconds() * ALIGNMENT_FACTOR)
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
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        // .insert_resource(WindowDescriptor {
        //     title: "Bevy - Movement".to_string(),
        //     width: WIDTH,
        //     height: HEIGHT,
        //     ..WindowDescriptor::default()
        // }) // For some reason, this doesn't do anything
        // Startup Things
        .add_startup_system(add_boids)
        .add_startup_system(setup_camera)
        .add_startup_system(setup_window) // IDK Why the window doesn't resize with the descriptor
        // Systems
        .add_system(move_boids_system)
        .add_system(border_system)
        .add_system(alignment_system)
        .add_system(cohesion_system)
        .add_system(separation_system)
        // Debug Systems
        .run();
}
