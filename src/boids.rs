use std::collections::HashMap;
use bevy::{prelude::*, math::Vec3Swizzles};
use rand::prelude::*;

pub const BOIDS: usize = 1000;
pub const CHASERS: usize = BOIDS / 100;

pub const CHUNK_RESOLUTION: usize = 20;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SimState {
    Running,
    Paused,
}

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

impl BoidFactors {
    fn to_flocking_factors(&self) -> FlockingFactors {
        FlockingFactors {
            vision: self.vision,
            cohesion: self.cohesion,
            alignment: self.alignment,
            separation: self.separation,
            collision_avoidance: self.collision_avoidance,
        }
    }
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

impl ChaserFactors {
    fn to_flocking_factors(&self) -> FlockingFactors {
        FlockingFactors {
            vision: self.vision,
            cohesion: self.cohesion,
            alignment: self.alignment,
            separation: self.separation,
            collision_avoidance: self.collision_avoidance,
        }
    }
}

struct FlockingFactors {
    vision: f32,
    cohesion: f32,
    alignment: f32,
    separation: f32,
    collision_avoidance: f32,
}

#[derive(Component, Clone, Debug, PartialEq)]
struct Direction(Vec2);

// Why no work when adding directly to vec2?
impl From<Vec2> for Direction {
    fn from(v: Vec2) -> Self {
        Direction(v)
    }
}

impl From<Direction> for Vec2 {
    fn from(d: Direction) -> Self {
        d.0
    }
}

impl Direction {
    fn lerp(&mut self, other: Vec2, t: f32) {
        self.0 = self.0.lerp(other, t).normalize();
    }
}

#[derive(Component, Clone, Debug, PartialEq)]
struct Boid;

#[derive(Component, Clone, Debug, PartialEq)]
struct Chaser;

struct ApplyForceEvent(Entity, Vec2, f32);

fn spawn_creature(
    rng: &mut ThreadRng,
    commands: &mut Commands,
    creature_type: &str,
    boid_factors: BoidFactors,
    chaser_factors: ChaserFactors,
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
            *boid_factors,
            *chaser_factors,
            screen_width,
            screen_height,
        );
    }
    for _ in 1..=CHASERS {
        spawn_creature(
            &mut rng,
            &mut commands,
            "chaser",
            *boid_factors,
            *chaser_factors,
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
        if transform.translation.x >= width / 2.0 {
            transform.translation.x = -width / 2.0 + 1.0;
        } else if transform.translation.x <= -width / 2.0 {
            transform.translation.x = width / 2.0 - 1.0;
        }
        if transform.translation.y >= height / 2.0 {
            transform.translation.y = -height / 2.0 + 1.0;
        } else if transform.translation.y <= -height / 2.0 {
            transform.translation.y = height / 2.0 - 1.0;
        }
    }
}

fn scare_system(
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    boids: Query<(Entity, &Transform), With<Boid>>,
    chasers: Query<&Transform, With<Chaser>>,
    boid_factors: Res<BoidFactors>,
    cache_grid: Res<CacheGrid>,
    windows: Res<Windows>,
) {
    let window = windows.get_primary().unwrap();
    let screen_width = window.width();
    let screen_height = window.height();
    for (id, trans_a) in boids.iter() {
        let possibles = cache_grid.get_possibles(
            screen_width, screen_height,
            trans_a.translation.xy(), boid_factors.vision);
        for id_b in possibles {
            let trans_b;
            match chasers.get(id_b) {
                Ok(transform) => trans_b = transform,
                Err(_) => continue,
            }
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < boid_factors.vision {
                let run_direction = (trans_a.translation.xy() - trans_b.translation.xy()).normalize();
                apply_force_event_handler.send(ApplyForceEvent(
                    id,
                    run_direction,
                    boid_factors.scare,
                ));
            }
        }
    }
}

fn chase_system(
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    chasers: Query<(Entity, &Transform), With<Chaser>>,
    boids: Query<&Transform, With<Boid>>,
    chaser_factors: Res<ChaserFactors>,
    cache_grid: Res<CacheGrid>,
    windows: Res<Windows>,
) {
    let window = windows.get_primary().unwrap();
    let screen_width = window.width();
    let screen_height = window.height();

    for (id, trans_a) in chasers.iter() {
        let mut closest_target = (0.0, None);
        let possibles = cache_grid.get_possibles(
            screen_width, screen_height,
            trans_a.translation.xy(), chaser_factors.vision);
        for id_b in possibles {
            let trans_b;
            match boids.get(id_b) {
                Ok(transform) => trans_b = transform,
                Err(_) => continue,
            }
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
            let chase_direction = (closest_trans.translation.xy() - trans_a.translation.xy()).normalize();
            apply_force_event_handler.send(ApplyForceEvent(
                id,
                chase_direction,
                chaser_factors.chase,
            ));
        }
    }
}

fn boid_flocking_system(
    boids: Query<(Entity, &Direction, &Transform, &Sprite), With<Boid>>,
    apply_force_event_handler: EventWriter<ApplyForceEvent>,
    boid_factors: Res<BoidFactors>,
    cache_grid: Res<CacheGrid>,
    windows: Res<Windows>,
) {
    let mut boid_map = HashMap::new();
    for boid in boids.iter() {
        boid_map.insert(boid.0, (boid.1, boid.2, boid.3));
    }
    send_flocking_forces(
        apply_force_event_handler,
        boid_map,
        cache_grid,
        boid_factors.to_flocking_factors(),
        windows,
    );
}

fn chaser_flocking_system(
    chasers: Query<(Entity, &Direction, &Transform, &Sprite), With<Chaser>>,
    apply_force_event_handler: EventWriter<ApplyForceEvent>,
    chaser_factors: Res<ChaserFactors>,
    cache_grid: Res<CacheGrid>,
    windows: Res<Windows>,
) {
    let mut chaser_map = HashMap::new();
    for chaser in chasers.iter() {
        chaser_map.insert(chaser.0, (chaser.1, chaser.2, chaser.3));
    }
    send_flocking_forces(
        apply_force_event_handler,
        chaser_map,
        cache_grid,
        chaser_factors.to_flocking_factors(),
        windows,
    );
}

fn send_flocking_forces(
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    creatures: HashMap<Entity, (&Direction, &Transform, &Sprite)>,
    cache_grid: Res<CacheGrid>,
    factors: FlockingFactors,
    windows: Res<Windows>,
) {
    let window = windows.get_primary().unwrap();
    let screen_width = window.width();
    let screen_height = window.height();

    let FlockingFactors {
        vision,
        cohesion,
        alignment,
        separation,
        collision_avoidance,
    } = factors;

    for id_a in creatures.keys() {
        let (_dir_a, trans_a, sprite_a) = creatures.get(id_a).unwrap();
        let pos_a = trans_a.translation.xy();

        let mut average_position = Vec2::ZERO; // Cohesion
        let mut average_direction = Vec2::ZERO; // Alignment
        let mut average_close_position = Vec2::ZERO; // Separation

        let mut vision_count = 0;
        let mut half_vision_count = 0;

        let possibles = cache_grid.get_possibles(
            screen_width, screen_height,
            pos_a,
            vision
        );
        for id_b in possibles {
            if !creatures.contains_key(&id_b) {
                continue;
            }
            let (dir_b, trans_b, _sprite_b) = creatures.get(&id_b).unwrap();
            if *id_a == id_b {
                continue;
            }
            let pos_b = trans_b.translation.xy();
            let distance = pos_a.distance(pos_b);
            if distance < vision {
                vision_count += 1;
                average_position += pos_b;
                average_direction += dir_b.0;
            }
            if distance < vision / 2.0 {
                half_vision_count += 1;
                average_close_position += pos_b;
            }
            if let Some(size) = sprite_a.custom_size {
                if distance < size.x * 2.0 {
                    let away_direction = (trans_a.translation.xy() - trans_b.translation.xy()).normalize();
                    apply_force_event_handler.send(ApplyForceEvent(
                        *id_a,
                        away_direction,
                        collision_avoidance,
                    ));
                }
            }
        }

        if vision_count > 0 {
            average_position /= vision_count as f32;
            average_direction /= vision_count as f32;
            let cohesion_force = (average_position - trans_a.translation.xy()).normalize();
            apply_force_event_handler.send(ApplyForceEvent(*id_a, cohesion_force, cohesion));
            apply_force_event_handler.send(ApplyForceEvent(
                *id_a,
                average_direction.normalize(),
                alignment,
            ));
        }
        if half_vision_count > 0 {
            average_close_position /= half_vision_count as f32;
            let separation_force = (trans_a.translation.xy() - average_close_position).normalize();
            apply_force_event_handler.send(ApplyForceEvent(*id_a, separation_force, separation));
        }
    }
}

fn update_factors_system(
    mut chaser_query: Query<&mut Sprite, (With<Chaser>, Without<Boid>)>,
    mut boid_query: Query<&mut Sprite, (With<Boid>, Without<Chaser>)>,
    chaser_factors: Res<ChaserFactors>,
    boid_factors: Res<BoidFactors>,
) {
    if boid_factors.is_changed() {
        for mut sprite in boid_query.iter_mut() {
            sprite.color = boid_factors.color;
            sprite.custom_size = Some(boid_factors.size);
        }
    }
    if chaser_factors.is_changed() {
        for mut sprite in chaser_query.iter_mut() {
            sprite.color = chaser_factors.color;
            sprite.custom_size = Some(chaser_factors.size);
        }
    }
}

fn apply_force_event_system(
    mut apply_force_event_handler: EventReader<ApplyForceEvent>,
    mut creature_query: Query<&mut Direction>,
    timer: Res<Time>,
) {
    let delta_time = timer.delta_seconds();
    for ApplyForceEvent(id, force, factor) in apply_force_event_handler.iter() {
        if let Ok(mut direction) = creature_query.get_mut(*id) {
            direction.lerp(*force, factor * delta_time);
        }
    }
}

fn handle_input_system(
    keys: Res<Input<KeyCode>>,
    cache_grid: Res<CacheGrid>,
    mut sim_state: ResMut<State<SimState>>,
) {
    if keys.just_pressed(KeyCode::P) {
        let current_sim_state = sim_state.current();
        let new_sim_state = match current_sim_state {
            SimState::Paused => SimState::Running,
            SimState::Running => SimState::Paused,
        };
        sim_state.set(new_sim_state).unwrap();
    }
    if keys.just_pressed(KeyCode::D) {
        for row in cache_grid.iter() {
            println!("{:?}", row);
        }
        println!();
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
                collision_avoidance: 3.5,
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
                collision_avoidance: 2.0,
                chase: 5.0,
            },
        }
    }
}

impl Plugin for BoidsPlugin {
    fn build(&self, app: &mut App) {
        // Insert Resources
        app.insert_resource(self.initial_boid_factors)
            .insert_resource(self.initial_chaser_factors)
            .add_event::<ApplyForceEvent>()
            .add_state(SimState::Running);

        // Start up
        app.add_startup_system(setup_creatures)
            .add_startup_system(setup_world);

        app.add_system_set(
            SystemSet::new()
                .label("sim_updates")
                .with_system(update_factors_system)
                .with_system(handle_input_system)
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("movement")
                .with_system(move_system)
                .with_system(wrap_borders_system)
                .before("caching")
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("caching")
                .with_system(update_cache_grid_system)
                .after("movement")
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("flocking")
                .label("force_adding")
                .with_system(boid_flocking_system)
                .with_system(chaser_flocking_system)
                .after("caching"),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("interactions")
                .label("force_adding")
                .with_system(scare_system)
                .with_system(chase_system)
                .after("caching"),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .with_system(apply_force_event_system)
                .after("force_adding"),
        );
    }
}

#[derive(Debug, Default)]
struct CacheGrid {
    grid: Vec<Vec<Vec<Entity>>>,
}

impl<'a> IntoIterator for &'a CacheGrid {
    type Item = &'a Vec<Vec<Entity>>;
    type IntoIter = CacheGridIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        CacheGridIterator { cache_grid: self, i: 0 }
    }
}

struct CacheGridIterator<'a> {
    cache_grid: &'a CacheGrid,
    i: usize,
}

impl<'a> Iterator for CacheGridIterator<'a> {
    type Item = &'a Vec<Vec<Entity>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.cache_grid.grid.len() {
            return None;
        }
        let result = Some(&self.cache_grid.grid[self.i]);
        self.i += 1;
        result
    }
}

impl CacheGrid {
    fn get_possibles(&self, screen_width: f32, screen_height: f32, position: Vec2, radius: f32) -> Vec<Entity> {
        let rows = self.grid.len();
        let cols = self.grid[0].len();

        let x = position.x;
        let y = -position.y; // What in the actual frick. Why does flipping the sign make it better?

        let x_begin = x - radius;
        let y_begin = y - radius;
        let i_begin = (((y_begin / screen_height + 0.5) * rows as f32) as usize).clamp(0, rows - 1);
        let j_begin = (((x_begin / screen_width + 0.5) * cols as f32) as usize).clamp(0, cols - 1);

        // A comment here to remind me of the bug I had that took 4 days to fix.
        // I forgot to multiply by rows and cols. I'm so dumb for that
        let i_to = (radius * 2.0 / screen_height * rows as f32).ceil() as usize;
        let j_to = (radius * 2.0 / screen_width * cols as f32).ceil() as usize;

        let i_end = (i_begin + i_to).clamp(0, rows - 1);
        let j_end = (j_begin + j_to).clamp(0, cols - 1);

        let mut possibles = vec![];

        for i in i_begin..=i_end {
            for j in j_begin..=j_end {
                possibles.extend(self.grid[i][j].iter());
            }
        }

        possibles
    }

    fn iter(&self) -> CacheGridIterator {
        CacheGridIterator {
            cache_grid: self,
            i: 0,
        }
    }
}

fn setup_world(mut commands: Commands) {
    commands.insert_resource(
        CacheGrid {
            grid: vec![vec![vec![]; 0]; 0],
        }
    );
}

fn update_cache_grid_system(
    creature_query: Query<(Entity, &Transform), Or<(With<Boid>, With<Chaser>)>>,
    mut cache_grid: ResMut<CacheGrid>,
    windows: Res<Windows>,
) {
    let window = windows.get_primary().unwrap();
    let screen_width = window.width();
    let screen_height = window.height();

    let rows = (screen_height / CHUNK_RESOLUTION as f32).ceil() as usize;
    let cols = (screen_width / CHUNK_RESOLUTION as f32).ceil() as usize;

    let mut new_grid = vec![vec![vec![]; cols]; rows];

    for (entity, transform) in creature_query.iter() {
        let x = transform.translation.x;
        let y = -transform.translation.y; // What in the actual frick. Why does flipping the sign make it better?
        let i = (((y / screen_height + 0.5) * rows as f32) as usize).clamp(0, rows - 1);
        let j = (((x / screen_width + 0.5) * cols as f32) as usize).clamp(0, cols - 1);
        new_grid[i][j].push(entity);
    }

    cache_grid.grid = new_grid;
}