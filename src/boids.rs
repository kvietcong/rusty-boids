use std::sync::mpsc;

use bevy::{
    input::{mouse::MouseButtonInput, ElementState},
    math::Vec3Swizzles,
    prelude::*,
    tasks::ComputeTaskPool,
    utils::{HashMap, HashSet},
};
use rand::prelude::*;

use crate::{Cursor, DebugState, IS_WASM};

pub const POPULATION_A: usize = if IS_WASM { 600 } else { 1600 };
pub const POPULATION_B: usize = if IS_WASM { 50 } else { 200 };
pub const POPULATION_C: usize = if IS_WASM { 50 } else { 200 };

pub const CHUNK_RESOLUTION: usize = 20;

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

#[derive(Debug)]
pub struct SpawnProperties {
    pub amount: usize,
    pub radius: f32,
    pub creature_type: CreatureType,
}

impl Default for SpawnProperties {
    fn default() -> Self {
        SpawnProperties {
            amount: 10,
            radius: 10.0,
            creature_type: CreatureType::A,
        }
    }
}

#[derive(Debug)]
pub struct KillProperties {
    pub radius: f32,
    pub creature_types: HashSet<CreatureType>,
}

impl Default for KillProperties {
    fn default() -> Self {
        KillProperties {
            radius: 100.0,
            creature_types: CreatureType::all().into_iter().collect(),
        }
    }
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
    fn update_entity(&mut self, entity: Entity, pos: Vec2) {
        let x = pos.x;
        let y = pos.y;

        let i = (y / CHUNK_RESOLUTION as f32) as i8;
        let j = (x / CHUNK_RESOLUTION as f32) as i8;

        if let Some((old_i, old_j)) = self.associations.get(&entity) {
            let old_i = *old_i;
            let old_j = *old_j;
            if i == old_i && j == old_j {
                return;
            }
            if let Some(set) = self.grid.get_mut(&(old_i, old_j)) {
                set.remove(&entity);
                if set.is_empty() {
                    self.grid.remove(&(old_i, old_j));
                }
            }
        }

        if !self.grid.contains_key(&(i, j)) {
            self.grid.insert((i, j), HashSet::default());
        }
        self.grid.get_mut(&(i, j)).unwrap().insert(entity);
        self.associations.insert(entity, (i, j));
    }

    fn get_nearby_entities(&self, position: Vec2, radius: f32) -> Vec<Entity> {
        let mut result = vec![];

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

        for i in i_begin..=i_end {
            for j in j_begin..=j_end {
                if let Some(set) = self.grid.get(&(i, j)) {
                    result.extend(set.iter());
                }
            }
        }

        result
    }
}

fn spawn_creature(
    x: f32,
    y: f32,
    direction_vector: Vec2,
    creature_type: CreatureType,
    all_factors: &HashMap<CreatureType, Factors>,
    commands: &mut Commands,
) {
    let factors = all_factors.get(&creature_type).unwrap();
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: factors.color,
                custom_size: Some(factors.size),
                ..Sprite::default()
            },
            transform: Transform {
                translation: Vec3::new(x, y, 0.0),
                rotation: Quat::from_rotation_z(direction_vector.y.atan2(direction_vector.x)),
                ..Transform::default()
            },
            ..SpriteBundle::default()
        })
        .insert(Direction(direction_vector))
        .insert(creature_type);
}

fn spawn_creature_randomly(
    rng: Option<&mut ThreadRng>,
    commands: &mut Commands,
    creature_type: CreatureType,
    all_factors: &HashMap<CreatureType, Factors>,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
) {
    let mut temp_rng;
    let rng = match rng {
        Some(rng) => rng,
        None => {
            temp_rng = rand::thread_rng();
            &mut temp_rng
        }
    };
    let x = rng.gen_range(min_x..=max_x);
    let y = rng.gen_range(min_y..=max_y);
    let direction_vector =
        Vec2::new(rng.gen::<f32>() * 2.0 - 1.0, rng.gen::<f32>() * 2.0 - 1.0).normalize();
    spawn_creature(x, y, direction_vector, creature_type, all_factors, commands);
}

fn spawn_creature_randomly_on_screen(
    rng: Option<&mut ThreadRng>,
    commands: &mut Commands,
    creature_type: CreatureType,
    all_factors: &HashMap<CreatureType, Factors>,
    screen_width: f32,
    screen_height: f32,
) {
    spawn_creature_randomly(
        rng,
        commands,
        creature_type,
        all_factors,
        -screen_width / 2.0,
        screen_width / 2.0,
        -screen_height / 2.0,
        screen_height / 2.0,
    );
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
        spawn_creature_randomly_on_screen(
            Some(&mut rng),
            &mut commands,
            CreatureType::A,
            all_factors,
            screen_width,
            screen_height,
        );
    }
    for _ in 0..POPULATION_B {
        spawn_creature_randomly_on_screen(
            Some(&mut rng),
            &mut commands,
            CreatureType::B,
            all_factors,
            screen_width,
            screen_height,
        );
    }
    for _ in 0..POPULATION_C {
        spawn_creature_randomly_on_screen(
            Some(&mut rng),
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

fn parallel_scare_system(
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    creatures: Query<(Entity, &Transform, &CreatureType)>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    compute_task_pool: Res<ComputeTaskPool>,
    cache_grid: Res<CacheGrid>,
) {
    let creature_vec = creatures.iter().collect::<Vec<_>>();
    let creatures_per_thread = creature_vec.len() / compute_task_pool.0.thread_num();

    let cache_grid = &cache_grid;
    let creatures = &creatures;
    let all_factors = &all_factors;

    compute_task_pool.scope(|scope| {
        let (sender, receiver) = mpsc::channel();

        for chunk in creature_vec.chunks(creatures_per_thread) {
            let sender = sender.clone();
            scope.spawn(async move {
                for (entity_a, transform_a, type_a) in chunk {
                    let entity_a = *entity_a;
                    let position_a = transform_a.translation.xy();
                    let factors = all_factors.get(type_a).unwrap();
                    for entity_b in cache_grid.get_nearby_entities(position_a, factors.vision) {
                        if entity_b == entity_a {
                            continue;
                        }
                        let (position_b, type_b) = match creatures.get(entity_b) {
                            Ok(creature) => (creature.1.translation.xy(), creature.2),
                            Err(_) => continue,
                        };
                        if !factors.scared_of.contains(&type_b) {
                            continue;
                        }
                        if position_a.distance(position_b) <= factors.vision {
                            let run_direction = (position_a - position_b).normalize();
                            sender
                                .send(ApplyForceEvent(entity_a, run_direction, factors.scare))
                                .unwrap();
                        }
                    }
                }
            });
        }

        drop(sender);
        receiver
            .iter()
            .for_each(|event| apply_force_event_handler.send(event));
    });
}

fn scare_system(
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    creatures: Query<(Entity, &Transform, &CreatureType)>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    cache_grid: Res<CacheGrid>,
) {
    // I use `for_each` because it's faster than `iter` according to the Bevy docs
    creatures.for_each(|(entity_a, transform_a, type_a)| {
        let position_a = transform_a.translation.xy();
        let factors = all_factors.get(type_a).unwrap();
        for entity_b in cache_grid.get_nearby_entities(position_a, factors.vision) {
            if entity_b == entity_a {
                continue;
            }
            let (position_b, type_b) = match creatures.get(entity_b) {
                Ok(creature) => (creature.1.translation.xy(), creature.2),
                Err(_) => continue,
            };
            if !factors.scared_of.contains(&type_b) {
                continue;
            }
            let distance = position_a.distance(position_b);
            if distance <= factors.vision {
                let run_direction = (position_a - position_b).normalize();
                apply_force_event_handler.send(ApplyForceEvent(
                    entity_a,
                    run_direction,
                    factors.scare,
                ));
            }
        }
    });
}

fn parallel_chase_system(
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    creatures: Query<(Entity, &Transform, &CreatureType)>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    compute_task_pool: Res<ComputeTaskPool>,
    cache_grid: Res<CacheGrid>,
) {
    let creature_vec = creatures.iter().collect::<Vec<_>>();
    let creatures_per_thread = creature_vec.len() / compute_task_pool.0.thread_num();

    let cache_grid = &cache_grid;
    let creatures = &creatures;
    let all_factors = &all_factors;

    compute_task_pool.scope(|scope| {
        let (sender, receiver) = mpsc::channel();

        for chunk in creature_vec.chunks(creatures_per_thread) {
            let sender = sender.clone();
            scope.spawn(async move {
                for (entity_a, transform_a, type_a) in chunk {
                    let id_a = *entity_a;
                    let position_a = transform_a.translation.xy();
                    let factors = all_factors.get(type_a).unwrap();
                    let mut closest_target = (0.0, None);
                    for entity_b in cache_grid.get_nearby_entities(position_a, factors.vision) {
                        if id_a == entity_b {
                            continue;
                        }
                        let (position_b, type_b) = match creatures.get(entity_b) {
                            Ok(creature) => (creature.1.translation.xy(), creature.2),
                            Err(_) => continue,
                        };
                        if !factors.will_chase.contains(&type_b) {
                            continue;
                        }
                        let distance = position_a.distance(position_b);
                        if distance <= factors.vision {
                            closest_target = match closest_target {
                                (_, None) => (distance, Some(position_b)),
                                (old_distance, Some(_)) => {
                                    if old_distance > distance {
                                        (distance, Some(position_b))
                                    } else {
                                        closest_target
                                    }
                                }
                            };
                        }
                    }

                    let closest_position = match closest_target {
                        (_, Some(position)) => position,
                        (_, None) => continue,
                    };
                    let chase_direction = (closest_position - position_a).normalize();
                    sender
                        .send(ApplyForceEvent(id_a, chase_direction, factors.chase))
                        .unwrap();
                }
            });
        }

        drop(sender);
        receiver
            .iter()
            .for_each(|event| apply_force_event_handler.send(event));
    });
}

fn chase_system(
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    creatures: Query<(Entity, &Transform, &CreatureType)>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    cache_grid: Res<CacheGrid>,
) {
    creatures.for_each(|(entity_a, transform_a, type_a)| {
        let position_a = transform_a.translation.xy();
        let factors = all_factors.get(type_a).unwrap();
        let mut closest_target = (0.0, None);

        for entity_b in cache_grid.get_nearby_entities(position_a, factors.vision) {
            let (position_b, type_b) = match creatures.get(entity_b) {
                Ok(creature) => (creature.1.translation.xy(), creature.2),
                Err(_) => continue,
            };
            if !factors.will_chase.contains(type_b) {
                continue;
            }
            let distance = position_a.distance(position_b);
            if distance <= factors.vision {
                closest_target = match closest_target {
                    (_, None) => (distance, Some(position_b)),
                    (old_distance, Some(_)) => {
                        if old_distance > distance {
                            (distance, Some(position_b))
                        } else {
                            closest_target
                        }
                    }
                };
            }
        }

        let closest_position = match closest_target {
            (_, Some(position)) => position,
            (_, None) => return,
        };
        let chase_direction = (closest_position - position_a).normalize();
        apply_force_event_handler.send(ApplyForceEvent(entity_a, chase_direction, factors.chase));
    });
}

fn parallel_flocking_system(
    creatures: Query<(Entity, &Direction, &Transform, &Sprite, &CreatureType)>,
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    compute_task_pool: Res<ComputeTaskPool>,
    cache_grid: Res<CacheGrid>,
) {
    let creature_vec = creatures.iter().collect::<Vec<_>>();
    let creatures_per_thread = creature_vec.len() / compute_task_pool.0.thread_num();

    let cache_grid = &cache_grid;
    let creatures = &creatures;
    let all_factors = &all_factors;

    compute_task_pool.scope(|scope| {
        let (sender, receiver) = mpsc::channel();

        for chunk in creature_vec.chunks(creatures_per_thread) {
            let sender = sender.clone();
            scope.spawn(async move {
                for (entity_a, _, transform_a, sprite_a, type_a) in chunk {
                    let entity_a = *entity_a;
                    let type_a = *type_a;
                    let factors = all_factors.get(type_a).unwrap();
                    let position_a = transform_a.translation.xy();

                    let mut average_position = Vec2::ZERO; // Cohesion
                    let mut average_direction = Vec2::ZERO; // Alignment
                    let mut average_close_position = Vec2::ZERO; // Separation

                    let mut vision_count = 0;
                    let mut half_vision_count = 0;

                    for entity_b in cache_grid.get_nearby_entities(position_a, factors.vision) {
                        if entity_a == entity_b {
                            continue;
                        }
                        let get_creature = creatures.get(entity_b);
                        if get_creature.is_err() {
                            continue;
                        }
                        let (_, direction_b, transform_b, _, type_b) = get_creature.unwrap();
                        if type_a != type_b {
                            continue;
                        }
                        let position_b = transform_b.translation.xy();
                        let distance = position_a.distance(position_b);
                        if distance <= factors.vision {
                            vision_count += 1;
                            average_position += position_b;
                            average_direction += direction_b.0;
                        }
                        if distance <= factors.vision / 2.0 {
                            half_vision_count += 1;
                            average_close_position += position_b;
                        }
                        let size = match sprite_a.custom_size {
                            Some(size) => size,
                            None => continue,
                        };
                        if distance <= size.x * 2.0 {
                            let away_direction = (position_a - position_b).normalize();
                            sender
                                .send(ApplyForceEvent(
                                    entity_a,
                                    away_direction,
                                    factors.collision_avoidance,
                                ))
                                .unwrap();
                        }
                    }

                    if vision_count > 0 {
                        average_position /= vision_count as f32;
                        average_direction /= vision_count as f32;
                        let cohesion_force =
                            (average_position - transform_a.translation.xy()).normalize();
                        sender
                            .send(ApplyForceEvent(entity_a, cohesion_force, factors.cohesion))
                            .unwrap();
                        sender
                            .send(ApplyForceEvent(
                                entity_a,
                                average_direction.normalize(),
                                factors.alignment,
                            ))
                            .unwrap();
                    }
                    if half_vision_count > 0 {
                        average_close_position /= half_vision_count as f32;
                        let separation_force = (position_a - average_close_position).normalize();
                        sender
                            .send(ApplyForceEvent(
                                entity_a,
                                separation_force,
                                factors.separation,
                            ))
                            .unwrap();
                    }
                }
            });
        }

        drop(sender);
        for apply_force_event in receiver {
            apply_force_event_handler.send(apply_force_event);
        }
    });
}

fn flocking_system(
    creatures: Query<(Entity, &Direction, &Transform, &Sprite, &CreatureType)>,
    mut apply_force_event_handler: EventWriter<ApplyForceEvent>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    cache_grid: Res<CacheGrid>,
) {
    creatures.for_each(|(entity_a, _, transform_a, sprite_a, type_a)| {
        let factors = all_factors.get(type_a).unwrap();
        let position_a = transform_a.translation.xy();

        let mut average_position = Vec2::ZERO; // Cohesion
        let mut average_direction = Vec2::ZERO; // Alignment
        let mut average_close_position = Vec2::ZERO; // Separation

        let mut vision_count = 0;
        let mut half_vision_count = 0;

        for entity_b in cache_grid.get_nearby_entities(position_a, factors.vision) {
            if entity_a == entity_b {
                continue;
            }
            let get_creature = creatures.get(entity_b);
            if get_creature.is_err() {
                continue;
            }
            let (_, direction_b, transform_b, _, type_b) = get_creature.unwrap();
            if type_a != type_b {
                continue;
            }
            let position_b = transform_b.translation.xy();
            let distance = position_a.distance(position_b);
            if distance <= factors.vision {
                vision_count += 1;
                average_position += position_b;
                average_direction += direction_b.0;
            }
            if distance <= factors.vision / 2.0 {
                half_vision_count += 1;
                average_close_position += position_b;
            }
            let size = match sprite_a.custom_size {
                Some(size) => size,
                None => continue,
            };
            if distance <= size.x * 2.0 {
                let away_direction = (position_a - position_b).normalize();
                apply_force_event_handler.send(ApplyForceEvent(
                    entity_a,
                    away_direction,
                    factors.collision_avoidance,
                ));
            }
        }

        if vision_count > 0 {
            average_position /= vision_count as f32;
            average_direction /= vision_count as f32;
            let cohesion_force = (average_position - position_a).normalize();
            apply_force_event_handler.send(ApplyForceEvent(
                entity_a,
                cohesion_force,
                factors.cohesion,
            ));
            apply_force_event_handler.send(ApplyForceEvent(
                entity_a,
                average_direction.normalize(),
                factors.alignment,
            ));
        }
        if half_vision_count > 0 {
            average_close_position /= half_vision_count as f32;
            let separation_force = (position_a - average_close_position).normalize();
            apply_force_event_handler.send(ApplyForceEvent(
                entity_a,
                separation_force,
                factors.separation,
            ));
        }
    });
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
    for ApplyForceEvent(entity, force, factor) in apply_force_event_handler.iter() {
        if let Ok(mut direction) = creature_query.get_mut(*entity) {
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
    creature_query: Query<(Entity, &Transform), Changed<Transform>>,
    mut cache_grid: ResMut<CacheGrid>,
) {
    for (entity, transform) in creature_query.iter() {
        cache_grid.update_entity(entity, transform.translation.xy());
    }
}

fn spawn_system(
    cursor: Res<Cursor>,
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    spawn_properties: Res<SpawnProperties>,
    all_factors: Res<HashMap<CreatureType, Factors>>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
) {
    for event in mouse_button_events.iter() {
        if event.button != MouseButton::Left
            || event.state != ElementState::Pressed
            || !keys.pressed(KeyCode::LShift)
        {
            continue;
        }
        let mut rng = rand::thread_rng();
        for _ in 0..spawn_properties.amount {
            spawn_creature_randomly(
                Some(&mut rng),
                &mut commands,
                spawn_properties.creature_type,
                all_factors.as_ref(),
                cursor.position.x - spawn_properties.radius,
                cursor.position.x + spawn_properties.radius,
                cursor.position.y - spawn_properties.radius,
                cursor.position.y + spawn_properties.radius,
            );
        }
    }
}
fn kill_system(
    cursor: Res<Cursor>,
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    kill_properties: Res<KillProperties>,
    creatures_query: Query<(Entity, &Transform, &CreatureType)>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
) {
    for event in mouse_button_events.iter() {
        if event.button != MouseButton::Left
            || event.state != ElementState::Pressed
            || !keys.pressed(KeyCode::LControl)
        {
            continue;
        }
        let min_x = cursor.position.x - kill_properties.radius;
        let max_x = cursor.position.x + kill_properties.radius;
        let min_y = cursor.position.y - kill_properties.radius;
        let max_y = cursor.position.y + kill_properties.radius;
        for (entity, transform, creature_type) in creatures_query.iter() {
            if transform.translation.x < min_x
                || transform.translation.x > max_x
                || transform.translation.y < min_y
                || transform.translation.y > max_y
                || !kill_properties.creature_types.contains(&creature_type)
            {
                continue;
            }
            commands.entity(entity).despawn();
        }
    }
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
            .insert_resource(CacheGrid::default())
            .insert_resource(SpawnProperties::default())
            .insert_resource(KillProperties::default())
            .add_event::<ApplyForceEvent>()
            .add_state(SimState::Running);

        // Start up
        app.add_startup_system(setup_creatures);

        app.add_system_set(
            SystemSet::new()
                .label("sim_updates")
                .with_system(update_factors_system)
                .with_system(handle_input_system)
                .with_system(kill_system)
                .with_system(spawn_system),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("movement")
                .with_system(move_system)
                .with_system(wrap_borders_system)
                .before("caching"),
        );

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .label("caching")
                .with_system(cache_grid_update_system)
                .after("movement"),
        );

        let flocking_system_set = SystemSet::on_update(SimState::Running)
            .label("flocking")
            .label("force_adding")
            .after("caching");

        if IS_WASM {
            app.add_system_set(flocking_system_set.with_system(flocking_system));
        } else {
            app.add_system_set(flocking_system_set.with_system(parallel_flocking_system));
        }

        let scare_chase_system_set = SystemSet::on_update(SimState::Running)
            .label("interactions")
            .label("force_adding")
            .after("caching");

        if IS_WASM {
            app.add_system_set(
                scare_chase_system_set
                    .with_system(scare_system)
                    .with_system(chase_system),
            );
        } else {
            app.add_system_set(
                scare_chase_system_set
                    .with_system(parallel_scare_system)
                    .with_system(parallel_chase_system),
            );
        }

        app.add_system_set(
            SystemSet::on_update(SimState::Running)
                .with_system(apply_force_event_system)
                .after("force_adding"),
        );

        app.add_system_set(SystemSet::on_update(DebugState::On));
    }
}
