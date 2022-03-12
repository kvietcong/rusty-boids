use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    utils::{HashMap, HashSet},
};
use rand::prelude::*;
use std::{sync::Arc, time::Instant};

use crate::DebugState;

// pub const POPULATION_A: usize = 800;
// pub const POPULATION_B: usize = 100;
// pub const POPULATION_C: usize = 100;
pub const POPULATION_A: usize = 5000;
pub const POPULATION_B: usize = 500;
pub const POPULATION_C: usize = 500;

pub const CHUNK_RESOLUTION: usize = 15;

#[derive(Debug, Default)]
struct DebugTimes(HashMap<String, Vec<u128>>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SimState {
    Running,
    Paused,
}

#[derive(Debug, Default, Clone)]
pub struct Factors {
    pub color: Color,
    pub speed: f32,
    pub vision: f32,
    pub size: Vec2,
    pub cohesion: f32,
    pub separation: f32,
    pub alignment: f32,
    pub collision_avoidance: f32,
    pub scare: f32,
    pub chase: f32,
    pub scared_of: HashSet<CreatureType>,
    pub will_chase: HashSet<CreatureType>,
}

#[derive(Clone, Debug, PartialEq, Copy, Component, Eq, Hash)]
pub enum CreatureType {
    A,
    B,
    C,
}

impl CreatureType {
    pub fn all() -> [CreatureType; 3] {
        [CreatureType::A, CreatureType::B, CreatureType::C]
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            CreatureType::A => "A",
            CreatureType::B => "B",
            CreatureType::C => "C",
        }
    }
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

struct ApplyForceEvent(Entity, Vec2, f32);

#[derive(Debug, Default)]
struct CacheGrid {
    grid: HashMap<(i8, i8), HashSet<Entity>>,
    associations: HashMap<Entity, (i8, i8)>,
}

impl CacheGrid {
    fn update(&mut self, id: Entity, pos: Vec2) {
        let x = pos.x;
        let y = pos.y;

        let i = (y / CHUNK_RESOLUTION as f32) as i8;
        let j = (x / CHUNK_RESOLUTION as f32) as i8;

        if let Some((old_i, old_j)) = self.associations.get(&id) {
            let old_i = *old_i;
            let old_j = *old_j;
            if i == old_i && j == old_j {
                return;
            }
            if let Some(set) = self.grid.get_mut(&(old_i, old_j)) {
                set.remove(&id);
                if set.is_empty() {
                    self.grid.remove(&(old_i, old_j));
                }
            }
        }

        if !self.grid.contains_key(&(i, j)) {
            self.grid.insert((i, j), HashSet::default());
        }
        self.grid.get_mut(&(i, j)).unwrap().insert(id);
        self.associations.insert(id, (i, j));
    }

    fn get_indices(&self, position: Vec2, radius: f32) -> (i8, i8, i8, i8) {
        let x = position.x;
        let y = position.y;

        let x_begin = x - radius;
        let y_begin = y - radius;
        let i_begin = (y_begin / CHUNK_RESOLUTION as f32) as i8;
        let j_begin = (x_begin / CHUNK_RESOLUTION as f32) as i8;

        let i_to = (radius * 2.0 / CHUNK_RESOLUTION as f32).ceil() as i8;
        let j_to = (radius * 2.0 / CHUNK_RESOLUTION as f32).ceil() as i8;

        let i_end = i_begin + i_to;
        let j_end = j_begin + j_to;

        (i_begin, i_end, j_begin, j_end)
    }

    fn get_possibles(&self, position: Vec2, radius: f32) -> Vec<Entity> {
        let mut possibles = Vec::with_capacity(800);
        let indices = self.get_indices(position, radius);
        let (i_begin, i_end, j_begin, j_end) = indices;
        for i in i_begin..=i_end {
            for j in j_begin..=j_end {
                if let Some(set) = self.grid.get(&(i, j)) {
                    possibles.extend(set.iter());
                }
            }
        }
        possibles
    }
}

fn spawn_creature(
    rng: &mut ThreadRng,
    commands: &mut Commands,
    creature_type: CreatureType,
    all_factors: &HashMap<CreatureType, Factors>,
    screen_width: f32,
    screen_height: f32,
) {
    let x = rng.gen_range(-screen_width / 2.0..screen_width / 2.0);
    let y = rng.gen_range(-screen_height / 2.0..screen_height / 2.0);
    let direction = Direction(
        Vec2::new(rng.gen::<f32>() * 2.0 - 1.0, rng.gen::<f32>() * 2.0 - 1.0).normalize(),
    );
    let factors = all_factors.get(&creature_type).unwrap();
    let mut creature_commands = commands.spawn();
    creature_commands
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: factors.color,
                custom_size: Some(factors.size),
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
        .insert(creature_type);
}

fn setup_creatures(
    windows: Res<Windows>,
    mut commands: Commands,
    all_factors: Res<HashMap<CreatureType, Factors>>,
) {
    let window = windows.get_primary().unwrap();
    let screen_width = window.width();
    let screen_height = window.height();

    let mut rng = rand::thread_rng();
    let all_factors = all_factors.as_ref();
    for _ in 0..POPULATION_A {
        spawn_creature(
            &mut rng,
            &mut commands,
            CreatureType::A,
            all_factors,
            screen_width,
            screen_height,
        );
    }
    for _ in 0..POPULATION_B {
        spawn_creature(
            &mut rng,
            &mut commands,
            CreatureType::B,
            all_factors,
            screen_width,
            screen_height,
        );
    }
    for _ in 0..POPULATION_C {
        spawn_creature(
            &mut rng,
            &mut commands,
            CreatureType::C,
            all_factors,
            screen_width,
            screen_height,
        );
    }
}

fn move_system(
    mut query: Query<(&mut Transform, &Direction, &CreatureType)>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    timer: Res<Time>,
) {
    for (mut transform, direction, creature_type) in query.iter_mut() {
        let speed = all_factors.get(creature_type).unwrap().speed;
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
    creatures: Query<(Entity, &Transform, &CreatureType)>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    cache_grid: Res<CacheGrid>,
) {
    for (id_a, trans_a, type_a) in creatures.iter() {
        let factors_a = all_factors.get(type_a).unwrap();
        let possibles = cache_grid.get_possibles(trans_a.translation.xy(), factors_a.vision);
        for id_b in possibles {
            let trans_b;
            if let Ok((_, transform, type_b)) = creatures.get(id_b) {
                if id_b == id_a || !factors_a.scared_of.contains(type_b) {
                    continue;
                }
                trans_b = transform;
            } else {
                continue;
            }
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < factors_a.vision {
                let run_direction =
                    (trans_a.translation.xy() - trans_b.translation.xy()).normalize();
                apply_force_event_handler.send(ApplyForceEvent(
                    id_a,
                    run_direction,
                    factors_a.scare,
                ));
            }
        }
    }
}

fn chase_system(
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    creatures: Query<(Entity, &Transform, &CreatureType)>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    cache_grid: Res<CacheGrid>,
    mut debug_times: ResMut<DebugTimes>,
) {
    let start = Instant::now();
    for (id_a, trans_a, type_a) in creatures.iter() {
        let factors_a = all_factors.get(type_a).unwrap();
        let mut closest_target = (0.0, None);
        let possibles = cache_grid.get_possibles(trans_a.translation.xy(), factors_a.vision);
        for id_b in possibles {
            let trans_b;
            if let Ok((_, trans, type_b)) = creatures.get(id_b) {
                if id_a == id_b || !factors_a.will_chase.contains(type_b) {
                    continue;
                }
                trans_b = trans;
            } else {
                continue;
            }
            let distance = trans_a.translation.distance(trans_b.translation);
            if distance < factors_a.vision {
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
            let chase_direction =
                (closest_trans.translation.xy() - trans_a.translation.xy()).normalize();
            apply_force_event_handler.send(ApplyForceEvent(id_a, chase_direction, factors_a.chase));
        }
    }
    debug_times
        .0
        .entry("chase".into())
        .or_insert(Vec::with_capacity(10000))
        .push(start.elapsed().as_micros());
}

fn flocking_system(
    creatures: Query<(Entity, &Direction, &Transform, &Sprite, &CreatureType)>,
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    cache_grid: Res<CacheGrid>,
    mut debug_times: ResMut<DebugTimes>,
) {
    let start = std::time::Instant::now();
    let (sender, receiver) = std::sync::mpsc::channel();

    let tx = sender.clone();
    let creatures = &creatures;
    let all_factors = all_factors.as_ref();
    crossbeam::scope(move |scope| {
        let mut handles = Vec::with_capacity(POPULATION_A + POPULATION_B + POPULATION_C);
        for (id_a, _dir_a, trans_a, sprite_a, type_a) in creatures.iter() {
            let tx = tx.clone();
            let creatures_arc = Arc::new(creatures);
            let factors_a_arc = Arc::new(all_factors.get(type_a).unwrap());

            let pos_a = trans_a.translation.xy();
            let pos_a_arc = Arc::new(pos_a);

            let possibles = cache_grid.get_possibles(pos_a, factors_a_arc.vision);
            let possibles_arc = Arc::new(possibles);

            let handle = scope.spawn(move |_| {
                let mut average_position = Vec2::ZERO; // Cohesion
                let mut average_direction = Vec2::ZERO; // Alignment
                let mut average_close_position = Vec2::ZERO; // Separation

                let mut vision_count = 0;
                let mut half_vision_count = 0;

                for id_b in possibles_arc.iter() {
                    let id_b = *id_b;
                    if let Ok((id_b, dir_b, trans_b, _sprite_b, type_b)) = creatures_arc.get(id_b) {
                        if id_a == id_b || type_a != type_b {
                            continue;
                        }
                        let pos_b = trans_b.translation.xy();
                        let distance = pos_a_arc.distance(pos_b);
                        if distance < factors_a_arc.vision {
                            vision_count += 1;
                            average_position += pos_b;
                            average_direction += dir_b.0;
                        }
                        if distance < factors_a_arc.vision / 2.0 {
                            half_vision_count += 1;
                            average_close_position += pos_b;
                        }
                        if let Some(size) = sprite_a.custom_size {
                            if distance < size.x * 2.0 {
                                let away_direction = (trans_a.translation.xy()
                                    - trans_b.translation.xy())
                                .normalize();
                                tx.send(Some(ApplyForceEvent(
                                    id_a,
                                    away_direction,
                                    factors_a_arc.collision_avoidance,
                                )))
                                .unwrap();
                            }
                        }
                    }
                }

                if vision_count > 0 {
                    average_position /= vision_count as f32;
                    average_direction /= vision_count as f32;
                    let cohesion_force = (average_position - trans_a.translation.xy()).normalize();
                    tx.send(Some(ApplyForceEvent(
                        id_a,
                        cohesion_force,
                        factors_a_arc.cohesion,
                    )))
                    .unwrap();
                    tx.send(Some(ApplyForceEvent(
                        id_a,
                        average_direction.normalize(),
                        factors_a_arc.alignment,
                    )))
                    .unwrap();
                }
                if half_vision_count > 0 {
                    average_close_position /= half_vision_count as f32;
                    let separation_force =
                        (trans_a.translation.xy() - average_close_position).normalize();
                    tx.send(Some(ApplyForceEvent(
                        id_a,
                        separation_force,
                        factors_a_arc.separation,
                    )))
                    .unwrap();
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.join().unwrap();
        }
    }).unwrap();
    sender.send(None).unwrap();

    for msg in receiver {
        match msg {
            Some(apply_force_event) => apply_force_event_handler.send(apply_force_event),
            None => break,
        }
    }

    debug_times
        .0
        .entry("flocking".into())
        .or_insert(Vec::with_capacity(10000))
        .push(start.elapsed().as_micros());
}

fn update_factors_system(
    mut creature_query: Query<(&CreatureType, &mut Sprite)>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
) {
    if all_factors.is_changed() {
        for (creature_type, mut sprite) in creature_query.iter_mut() {
            let factors = all_factors.get(creature_type).unwrap();
            sprite.color = factors.color;
            sprite.custom_size = Some(factors.size);
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

fn handle_input_system(keys: Res<Input<KeyCode>>, mut sim_state: ResMut<State<SimState>>) {
    if keys.just_pressed(KeyCode::P) {
        let current_sim_state = sim_state.current();
        let new_sim_state = match current_sim_state {
            SimState::Paused => SimState::Running,
            SimState::Running => SimState::Paused,
        };
        sim_state.set(new_sim_state).unwrap();
    }
}

fn cache_grid_update_system(
    creature_query: Query<(Entity, &Transform, &CreatureType), Changed<Transform>>,
    mut cache_grid: ResMut<CacheGrid>,
    mut debug_times: ResMut<DebugTimes>,
) {
    let start = Instant::now();
    for (entity, transform, _creature_type) in creature_query.iter() {
        cache_grid.update(entity, transform.translation.xy());
    }
    debug_times
        .0
        .entry("cache".into())
        .or_insert(Vec::with_capacity(10000))
        .push(start.elapsed().as_micros());
}

fn debug_time_system(debug_times: Res<DebugTimes>) {
    if debug_times.0.keys().len() <= 0 {
        return;
    }
    println!();
    let mut time_info: Vec<(&String, &Vec<u128>)> = debug_times.0.iter().collect();
    time_info.sort_unstable_by(|a, b| a.0.cmp(b.0));
    time_info.iter().for_each(|(system, times)| {
        println!(
            "{}: {}",
            system,
            times.iter().sum::<u128>() as f32 / times.len() as f32
        );
    });
}

pub struct BoidsPlugin {
    initial_factors: HashMap<CreatureType, Factors>,
}

impl Default for BoidsPlugin {
    fn default() -> Self {
        let mut initial_factors = HashMap::default();

        let mut a_scared_of = HashSet::default();
        a_scared_of.insert(CreatureType::B);
        a_scared_of.insert(CreatureType::C);
        initial_factors.insert(
            CreatureType::A,
            Factors {
                color: Color::WHITE,
                speed: 70.0,
                vision: 20.0,
                size: Vec2::new(4.0, 1.0),
                cohesion: 1.0,
                separation: 1.0,
                alignment: 3.0,
                collision_avoidance: 3.5,
                scare: 10.0,
                chase: 0.0,
                scared_of: a_scared_of,
                will_chase: HashSet::default(),
            },
        );

        let mut b_will_chase = HashSet::default();
        b_will_chase.insert(CreatureType::A);
        b_will_chase.insert(CreatureType::C);
        initial_factors.insert(
            CreatureType::B,
            Factors {
                color: Color::RED,
                speed: 60.0,
                vision: 30.0,
                size: Vec2::new(8.0, 4.0),
                cohesion: 0.5,
                separation: 0.5,
                alignment: 2.0,
                collision_avoidance: 2.0,
                scare: 0.0,
                chase: 2.0,
                scared_of: HashSet::default(),
                will_chase: b_will_chase,
            },
        );

        let mut c_scared_of = HashSet::default();
        c_scared_of.insert(CreatureType::B);
        let mut c_will_chase = HashSet::default();
        c_will_chase.insert(CreatureType::A);
        initial_factors.insert(
            CreatureType::C,
            Factors {
                color: Color::GOLD,
                speed: 65.0,
                vision: 25.0,
                size: Vec2::new(6.0, 2.0),
                cohesion: 0.75,
                separation: 0.75,
                alignment: 2.5,
                collision_avoidance: 3.0,
                scare: 5.0,
                chase: 1.0,
                scared_of: c_scared_of,
                will_chase: c_will_chase,
            },
        );

        Self { initial_factors }
    }
}

impl Plugin for BoidsPlugin {
    fn build(&self, app: &mut App) {
        // Insert Resources
        app.insert_resource(self.initial_factors.clone())
            .insert_resource(CacheGrid {
                ..Default::default()
            })
            .insert_resource(DebugTimes::default())
            .add_event::<ApplyForceEvent>()
            .add_state(SimState::Running);

        // Start up
        app.add_startup_system(setup_creatures);

        app.add_system_set(
            SystemSet::new()
                .label("sim_updates")
                .with_system(update_factors_system)
                .with_system(handle_input_system),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("movement")
                .before("caching")
                .with_system(move_system)
                .with_system(wrap_borders_system),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("caching")
                .after("movement")
                .with_system(cache_grid_update_system),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("flocking")
                .label("force_adding")
                .after("caching")
                .with_system(flocking_system),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("interactions")
                .label("force_adding")
                .after("caching")
                .with_system(scare_system)
                .with_system(chase_system),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .after("force_adding")
                .with_system(apply_force_event_system),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("cleanup")
                .after("force_adding"),
        );

        app.add_system_set(SystemSet::on_update(DebugState::On).with_system(debug_time_system));
    }
}
