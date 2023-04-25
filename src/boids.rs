use std::sync::{Arc, Mutex};

use bevy::{
    input::mouse::MouseButtonInput,
    math::Vec3Swizzles,
    prelude::*,
    tasks::ComputeTaskPool,
    utils::{HashMap, HashSet},
    window::PrimaryWindow,
};
use rand::prelude::*;

use crate::{ui::UiPlugin, Cursor, IS_WASM};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Resource)]
pub struct Features {
    pub chasing: bool,
    pub running: bool,
    pub killing: bool,
    pub flocking: bool,
    pub reproduction: bool,
    pub energy_draining: bool,
}

impl Default for Features {
    fn default() -> Self {
        Features {
            chasing: true,
            running: true,
            flocking: true,
            killing: false,
            reproduction: false,
            energy_draining: false,
        }
    }
}

pub const INITIAL_POPULATIONS: [usize; 3] = [
    if IS_WASM { 500 } else { 1000 },
    if IS_WASM { 50 } else { 200 },
    if IS_WASM { 50 } else { 300 },
];

pub const CHUNK_RESOLUTION: usize = 20;

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum SimState {
    #[default]
    Running,
    Paused,
}

#[derive(Debug, Clone, Resource)]
pub struct Factors {
    pub color: Color,
    pub speed: f32,
    pub vision: f32,
    pub size: f32,
    pub cohesion: f32,
    pub separation: f32,
    pub alignment: f32,
    pub collision_avoidance: f32,
    pub scare: f32,
    pub chase: f32,
    pub max_energy: f32,
    pub fertility_cooldown: f32,
    pub predator_of: HashSet<CreatureType>,
}

impl Default for Factors {
    fn default() -> Self {
        Self {
            color: Color::PINK,
            speed: 70.0,
            vision: 15.0,
            size: 6.0,
            cohesion: 1.0,
            separation: 1.0,
            alignment: 3.0,
            collision_avoidance: 4.0,
            scare: 5.0,
            chase: 5.0,
            max_energy: 100.0,
            fertility_cooldown: 15.0,
            predator_of: HashSet::default(),
        }
    }
}

#[derive(Debug, Resource)]
pub struct SpawnProperties {
    pub amount: usize,
    pub radius: f32,
}

impl Default for SpawnProperties {
    fn default() -> Self {
        SpawnProperties {
            amount: 10,
            radius: 10.0,
        }
    }
}

#[derive(Debug, Resource)]
pub struct DespawnProperties {
    pub radius: f32,
}

impl Default for DespawnProperties {
    fn default() -> Self {
        DespawnProperties { radius: 100.0 }
    }
}

// TODO: Maybe generalize this?
#[derive(Clone, Debug, PartialEq, Copy, Component, Eq, Hash, Resource)]
pub struct CreatureType(pub usize);

impl Default for CreatureType {
    fn default() -> Self {
        CreatureType(0)
    }
}

impl From<usize> for CreatureType {
    fn from(val: usize) -> Self {
        CreatureType(val)
    }
}

impl std::fmt::Display for CreatureType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl CreatureType {
    pub fn to_string(&self) -> String {
        format!("Type {}", self.0)
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

#[derive(Debug, Clone, PartialEq, Component)]
pub struct Fertility {
    pub time_till_fertile: f32,
    pub amount: usize,
}

#[derive(Debug, Clone, PartialEq, Component, PartialOrd)]
pub struct Energy(f32);

struct ApplyForceEvent(Entity, Vec2, f32);

struct EnergyChangeEvent(Entity, f32);

#[derive(Debug, Resource, Default)]
pub struct FactorInfo {
    pub factors: HashMap<CreatureType, Factors>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum SystemStages {
    Spawn,
    Calculate,
    Apply,
    Act,
    Cache,
}

#[derive(Debug, Default, Resource)]
struct HashGrid {
    grid: HashMap<(i8, i8), HashSet<Entity>>,
    associations: HashMap<Entity, (i8, i8)>,
}

impl HashGrid {
    fn update_entity(&mut self, entity: Entity, pos: Vec2) {
        let i = (pos.y / CHUNK_RESOLUTION as f32) as i8;
        let j = (pos.x / CHUNK_RESOLUTION as f32) as i8;

        // Note: `associations` could be extra overhead compared to the entity storing it.
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

        self.grid
            .entry((i, j))
            .or_insert(HashSet::default())
            .insert(entity);
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
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: factors.color,
                custom_size: Some(Vec2::splat(factors.size)),
                ..Sprite::default()
            },
            transform: Transform {
                translation: Vec3::new(x, y, 0.0),
                rotation: Quat::from_rotation_z(-direction_vector.x.atan2(direction_vector.y)),
                ..Transform::default()
            },
            ..SpriteBundle::default()
        })
        .insert(Direction(direction_vector))
        .insert(Energy(factors.max_energy))
        .insert(Fertility {
            time_till_fertile: factors.fertility_cooldown,
            amount: 1,
        })
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
    mut commands: Commands,
    factor_info: Res<FactorInfo>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
) {
    let window = primary_query.get_single().unwrap();
    let screen_width = window.width();
    let screen_height = window.height();

    let mut rng = rand::thread_rng();
    INITIAL_POPULATIONS
        .into_iter()
        .enumerate()
        .for_each(|(index, population_size)| {
            let creature_type = CreatureType(index);
            for _ in 0..population_size {
                spawn_creature_randomly_on_screen(
                    Some(&mut rng),
                    &mut commands,
                    creature_type,
                    &factor_info.factors,
                    screen_width,
                    screen_height,
                );
            }
        });
}

fn move_system(
    mut query: Query<(&mut Transform, &Direction, &CreatureType)>,
    factor_info: Res<FactorInfo>,
    timer: Res<Time>,
) {
    for (mut transform, direction, creature_type) in query.iter_mut() {
        let speed = factor_info.factors.get(creature_type).unwrap().speed;
        transform.translation.x += direction.0.x * speed * timer.delta_seconds();
        transform.translation.y += direction.0.y * speed * timer.delta_seconds();
        transform.rotation = Quat::from_rotation_z(-direction.0.x.atan2(direction.0.y));
    }
}

fn wrap_borders_system(
    mut query: Query<&mut Transform>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
) {
    let window = primary_query.get_single().unwrap();
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

fn flocking_system(
    creatures: Query<(Entity, &Direction, &Transform, &CreatureType)>,
    apply_force_event_handler: EventWriter<ApplyForceEvent>,
    factor_info: Res<FactorInfo>,
    hash_grid: Res<HashGrid>,
    features: Res<Features>,
) {
    if !features.flocking && !features.chasing && !features.running {
        return;
    }

    let compute_task_pool = ComputeTaskPool::get();
    let creature_vec = creatures.iter().collect::<Vec<_>>();
    let creatures_per_thread = creature_vec.len() / compute_task_pool.thread_num();
    if creatures_per_thread == 0 {
        return;
    }

    let features = &features;
    let hash_grid = &hash_grid;
    let creatures = &creatures;
    let factor_info = &factor_info;
    let apply_force_event_handler = Arc::new(Mutex::new(apply_force_event_handler));

    compute_task_pool.scope(|scope| {
        for chunk in creature_vec.chunks(creatures_per_thread) {
            let apply_force_event_handler = apply_force_event_handler.clone();
            scope.spawn(async move {
                for (entity_a, _, transform_a, type_a) in chunk {
                    let entity_a = *entity_a;
                    let type_a = *type_a;
                    let factors_a = factor_info.factors.get(type_a).unwrap();
                    let position_a = transform_a.translation.xy();

                    let mut average_position = Vec2::ZERO; // Cohesion
                    let mut average_direction = Vec2::ZERO; // Alignment
                    let mut average_close_position = Vec2::ZERO; // Separation

                    let mut vision_count = 0;
                    let mut half_vision_count = 0;
                    let mut closest_target = (0.0, None);

                    for entity_b in hash_grid.get_nearby_entities(position_a, factors_a.vision) {
                        let (_, direction_b, transform_b, type_b) = if entity_a != entity_b {
                            let Ok(creature) = creatures.get(entity_b) else { continue; };
                            creature
                        } else {
                            continue;
                        };

                        let position_b = transform_b.translation.xy();
                        let distance = position_a.distance(position_b);

                        // Flocking
                        if type_a == type_b && features.flocking {
                            if distance <= factors_a.vision {
                                vision_count += 1;
                                average_position += position_b;
                                average_direction += direction_b.0;
                            }
                            if distance <= factors_a.vision / 2.0 {
                                half_vision_count += 1;
                                average_close_position += position_b;
                            }
                            if distance <= factors_a.size * 2.0 {
                                let away_direction = (position_a - position_b).normalize();
                                apply_force_event_handler
                                    .lock()
                                    .unwrap()
                                    .send(ApplyForceEvent(
                                        entity_a,
                                        away_direction,
                                        factors_a.collision_avoidance,
                                    ));
                            }
                        }

                        // Chase
                        if factors_a.predator_of.contains(&type_b) && features.chasing {
                            if distance <= factors_a.vision {
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

                        // Run
                        if features.running {
                            let factors_b = factor_info.factors.get(type_b).unwrap();
                            if factors_b.predator_of.contains(&type_a) {
                                if distance <= factors_a.vision {
                                    let run_direction = (position_a - position_b).normalize();
                                    apply_force_event_handler.lock().unwrap().send(
                                        ApplyForceEvent(entity_a, run_direction, factors_a.scare),
                                    );
                                }
                            }
                        }
                    }

                    if vision_count > 0 && features.flocking {
                        average_position /= vision_count as f32;
                        average_direction /= vision_count as f32;
                        let cohesion_force =
                            (average_position - transform_a.translation.xy()).normalize();
                        apply_force_event_handler
                            .lock()
                            .unwrap()
                            .send(ApplyForceEvent(
                                entity_a,
                                cohesion_force,
                                factors_a.cohesion,
                            ));
                        apply_force_event_handler
                            .lock()
                            .unwrap()
                            .send(ApplyForceEvent(
                                entity_a,
                                average_direction.normalize(),
                                factors_a.alignment,
                            ));
                    }
                    if half_vision_count > 0 && features.flocking {
                        average_close_position /= half_vision_count as f32;
                        let separation_force = (position_a - average_close_position).normalize();
                        apply_force_event_handler
                            .lock()
                            .unwrap()
                            .send(ApplyForceEvent(
                                entity_a,
                                separation_force,
                                factors_a.separation,
                            ));
                    }

                    // Chase
                    let closest_position = match closest_target {
                        (_, Some(position)) => position,
                        (_, None) => continue,
                    };
                    let chase_direction = (closest_position - position_a).normalize();
                    apply_force_event_handler
                        .lock()
                        .unwrap()
                        .send(ApplyForceEvent(entity_a, chase_direction, factors_a.chase));
                }
            });
        }
    });
}

fn update_factors_system(
    mut creature_query: Query<(&CreatureType, &mut Sprite)>,
    factor_info: Res<FactorInfo>,
) {
    if factor_info.is_changed() {
        for (creature_type, mut sprite) in creature_query.iter_mut() {
            let factors = factor_info.factors.get(creature_type).unwrap();
            sprite.color = factors.color;
            sprite.custom_size = Some(Vec2::splat(factors.size));
        }
    }
}

fn apply_forces_system(
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

fn pause_system(
    keys: Res<Input<KeyCode>>,
    sim_state: Res<State<SimState>>,
    mut next_sim_state: ResMut<NextState<SimState>>,
) {
    if keys.just_pressed(KeyCode::P) {
        let new_sim_state = match sim_state.0 {
            SimState::Running => SimState::Paused,
            _ => SimState::Running,
        };
        println!("{:?} to {:?}", sim_state, new_sim_state);
        next_sim_state.set(new_sim_state);
    }
}

fn hash_grid_update_system(
    creature_query: Query<(Entity, &Transform), Changed<Transform>>,
    mut hash_grid: ResMut<HashGrid>,
) {
    for (entity, transform) in creature_query.iter() {
        hash_grid.update_entity(entity, transform.translation.xy());
    }
}

fn spawn_system(
    cursor: Res<Cursor>,
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    factor_info: Res<FactorInfo>,
    spawn_properties: Res<SpawnProperties>,
    selected_creature_type: Res<CreatureType>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
) {
    for event in mouse_button_events.iter() {
        if event.button != MouseButton::Left
            || event.state.is_pressed()
            || !keys.pressed(KeyCode::LShift)
        {
            continue;
        }
        let mut rng = rand::thread_rng();
        for _ in 0..spawn_properties.amount {
            spawn_creature_randomly(
                Some(&mut rng),
                &mut commands,
                *selected_creature_type,
                &factor_info.factors,
                cursor.position.x - spawn_properties.radius,
                cursor.position.x + spawn_properties.radius,
                cursor.position.y - spawn_properties.radius,
                cursor.position.y + spawn_properties.radius,
            );
        }
    }
}

fn despawn_system(
    cursor: Res<Cursor>,
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    despawn_properties: Res<DespawnProperties>,
    selected_creature_type: Res<CreatureType>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
    creatures_query: Query<(Entity, &Transform, &CreatureType)>,
) {
    for event in mouse_button_events.iter() {
        if event.button != MouseButton::Left
            || event.state.is_pressed()
            || !keys.pressed(KeyCode::LControl)
        {
            continue;
        }
        let min_x = cursor.position.x - despawn_properties.radius;
        let max_x = cursor.position.x + despawn_properties.radius;
        let min_y = cursor.position.y - despawn_properties.radius;
        let max_y = cursor.position.y + despawn_properties.radius;
        for (entity, transform, &creature_type) in creatures_query.iter() {
            if transform.translation.x >= min_x
                && transform.translation.x <= max_x
                && transform.translation.y >= min_y
                && transform.translation.y <= max_y
                && *selected_creature_type == creature_type
            {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn kill_system(
    mut commands: Commands,
    features: Res<Features>,
    hash_grid: Res<HashGrid>,
    factor_info: Res<FactorInfo>,
    creatures: Query<(Entity, &Transform, &CreatureType, &Energy)>,
    mut energy_change_event_handler: EventWriter<EnergyChangeEvent>,
) {
    if !features.killing {
        return;
    }
    creatures.for_each(|(entity_a, transform_a, type_a, energy_a)| {
        let position_a = transform_a.translation.xy();
        let factors_a = factor_info.factors.get(type_a).unwrap();

        for entity_b in hash_grid.get_nearby_entities(position_a, factors_a.size) {
            if entity_b == entity_a {
                continue;
            }
            let (position_b, type_b, energy_b) = match creatures.get(entity_b) {
                Ok(creature) => (creature.1.translation.xy(), creature.2, creature.3),
                Err(_) => continue,
            };
            let factors_b = factor_info.factors.get(type_b).unwrap();

            let is_a_predator = factors_a.predator_of.contains(type_b);
            let is_b_predator = factors_b.predator_of.contains(type_a);
            if position_a.distance(position_b) <= factors_a.size + factors_b.size {
                // This ternary is disgusting
                let (killed_entity, killer_entity) = if is_a_predator && is_b_predator {
                    if energy_a > energy_b {
                        (entity_a, entity_b)
                    } else if energy_a > energy_b {
                        (entity_b, entity_a)
                    } else {
                        continue;
                    }
                } else if is_a_predator {
                    (entity_b, entity_a)
                } else if is_b_predator {
                    (entity_a, entity_b)
                } else {
                    continue;
                };
                energy_change_event_handler.send(EnergyChangeEvent(killer_entity, 4.0));
                commands.entity(killed_entity).despawn();
            }
        }
    });
}

fn reproduction_system(
    timer: Res<Time>,
    mut commands: Commands,
    features: Res<Features>,
    hash_grid: Res<HashGrid>,
    factor_info: Res<FactorInfo>,
    mut energy_change_event_handler: EventWriter<EnergyChangeEvent>,
    mut creatures: Query<(Entity, &Transform, &CreatureType, &mut Fertility)>,
) {
    if !features.reproduction {
        return;
    }
    let mut reproducers = HashSet::default();
    reproducers.reserve(1000);
    creatures.for_each(|(entity_a, transform_a, type_a, fertility)| {
        let position_a = transform_a.translation.xy();
        let factors = factor_info.factors.get(type_a).unwrap();

        for entity_b in hash_grid.get_nearby_entities(position_a, factors.vision) {
            if entity_b == entity_a {
                continue;
            }
            let (position_b, type_b) = match creatures.get(entity_b) {
                Ok(creature) => (creature.1.translation.xy(), creature.2),
                Err(_) => continue,
            };
            if position_a.distance(position_b) <= factors.size * 2.0
                && type_a == type_b
                && !reproducers.contains(&entity_a)
                && !reproducers.contains(&entity_b)
                && fertility.time_till_fertile <= 0.0
            {
                let spawn_radius = 15.0;
                for _ in 0..fertility.amount {
                    spawn_creature_randomly(
                        None,
                        &mut commands,
                        *type_a,
                        &factor_info.factors,
                        position_a.x - spawn_radius,
                        position_a.x + spawn_radius,
                        position_a.y - spawn_radius,
                        position_a.y + spawn_radius,
                    );
                }
                reproducers.insert(entity_a);
                energy_change_event_handler.send(EnergyChangeEvent(entity_a, -15.0));
            }
        }
    });

    let delta_seconds = timer.delta_seconds();
    creatures.for_each_mut(|(entity, _, creature_type, mut fertility)| {
        if reproducers.contains(&entity) {
            fertility.time_till_fertile = factor_info
                .factors
                .get(creature_type)
                .unwrap()
                .fertility_cooldown;
        } else {
            fertility.time_till_fertile -= delta_seconds;
            fertility.time_till_fertile = fertility.time_till_fertile.max(0.0);
        }
    });
}

fn energy_drain_system(
    timer: Res<Time>,
    features: Res<Features>,
    creatures: Query<Entity, With<Energy>>,
    mut energy_change_event_handler: EventWriter<EnergyChangeEvent>,
) {
    if !features.energy_draining {
        return;
    }
    let delta_seconds = timer.delta_seconds();
    let burn_rate = 2f32;
    creatures.for_each(|entity| {
        energy_change_event_handler.send(EnergyChangeEvent(entity, -delta_seconds * burn_rate))
    });
}

fn apply_energy_change_system(
    mut commands: Commands,
    factor_info: Res<FactorInfo>,
    mut creature_query: Query<(Entity, &mut Energy, &CreatureType)>,
    mut energy_change_even_handler: EventReader<EnergyChangeEvent>,
) {
    for EnergyChangeEvent(entity, change) in energy_change_even_handler.iter() {
        if let Ok((entity, mut energy, creature_type)) = creature_query.get_mut(*entity) {
            let factors = factor_info.factors.get(creature_type).unwrap();
            energy.0 += change;
            energy.0 = energy.0.clamp(0.0, factors.max_energy);
            if energy.0 <= 0.0 {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub struct BoidsPlugin {
    initial_factors: HashMap<CreatureType, Factors>,
}

impl Default for BoidsPlugin {
    fn default() -> Self {
        let mut initial_factors = HashMap::default();

        initial_factors.insert(
            CreatureType(0),
            Factors {
                color: Color::CYAN,
                speed: 70.0,
                vision: 20.0,
                size: 1.0,
                cohesion: 1.0,
                separation: 1.0,
                alignment: 3.0,
                collision_avoidance: 3.5,
                scare: 10.0,
                chase: 0.0,
                max_energy: 50.0,
                fertility_cooldown: 10.0,
                predator_of: HashSet::default(),
                ..Default::default()
            },
        );

        let mut b_predator_of = HashSet::default();
        b_predator_of.insert(CreatureType(0));
        b_predator_of.insert(CreatureType(2));
        initial_factors.insert(
            CreatureType(1),
            Factors {
                color: Color::RED,
                speed: 60.0,
                vision: 30.0,
                size: 3.0,
                cohesion: 0.5,
                separation: 0.5,
                alignment: 2.0,
                collision_avoidance: 2.0,
                scare: 0.0,
                chase: 2.0,
                max_energy: 35.0,
                fertility_cooldown: 20.0,
                predator_of: b_predator_of,
                ..Default::default()
            },
        );

        let mut c_predator_of = HashSet::default();
        c_predator_of.insert(CreatureType(0));
        initial_factors.insert(
            CreatureType(2),
            Factors {
                color: Color::WHITE,
                speed: 65.0,
                vision: 25.0,
                size: 2.0,
                cohesion: 0.75,
                separation: 0.75,
                alignment: 2.5,
                collision_avoidance: 3.0,
                scare: 5.0,
                chase: 1.0,
                max_energy: 50.0,
                fertility_cooldown: 15.0,
                predator_of: c_predator_of,
                ..Default::default()
            },
        );

        Self { initial_factors }
    }
}

impl Plugin for BoidsPlugin {
    fn build(&self, app: &mut App) {
        // Insert Resources
        app.insert_resource(FactorInfo {
            factors: self.initial_factors.clone(),
        })
        .insert_resource(Features::default())
        .insert_resource(HashGrid::default())
        .insert_resource(CreatureType::default())
        .insert_resource(DespawnProperties::default())
        .insert_resource(SpawnProperties::default())
        .add_event::<ApplyForceEvent>()
        .add_event::<EnergyChangeEvent>()
        .add_state::<SimState>()
        .add_plugin(UiPlugin::default())
        .add_startup_system(setup_creatures)
        .configure_sets((
            SystemStages::Spawn,
            SystemStages::Calculate,
            SystemStages::Apply,
            SystemStages::Act,
            SystemStages::Cache,
        ))
        .add_systems((update_factors_system, pause_system))
        .add_systems(
            (despawn_system, spawn_system, kill_system)
                .in_set(SystemStages::Spawn)
                .in_set(OnUpdate(SimState::Running)),
        )
        .add_systems(
            (flocking_system, energy_drain_system, reproduction_system)
                .in_set(SystemStages::Calculate)
                .in_set(OnUpdate(SimState::Running)),
        )
        .add_systems(
            (apply_forces_system, apply_energy_change_system)
                .in_set(SystemStages::Apply)
                .in_set(OnUpdate(SimState::Running)),
        )
        .add_systems(
            (move_system, wrap_borders_system)
                .in_set(SystemStages::Act)
                .in_set(OnUpdate(SimState::Running)),
        )
        .add_system(hash_grid_update_system.in_set(SystemStages::Cache));
    }
}
