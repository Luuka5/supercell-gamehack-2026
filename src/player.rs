use crate::arena::{ArenaConfig, ArenaGrid, Collectible};
use crate::pathfinding::has_line_of_sight;
use bevy::prelude::*;

pub const PLAYER_SPEED: f32 = 20.0;
pub const ACCELERATION: f32 = 10.0;
pub const DECELERATION: f32 = 30.0;

pub const MAX_HP: u32 = 3;
pub const TURRET_COOLDOWN: f32 = 4.0;
pub const TURRET_DAMAGE: u32 = 1;

const PLAYER_SIZE: f32 = 1.0;

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct MovementController {
    pub input_direction: Vec3,
    pub rotation_delta: f32,
    pub current_velocity: Vec3,
}

#[derive(Component, Default)]
pub struct PlayerStatus {
    pub visible_players: Vec<Entity>,
}

#[derive(Component, Default)]
pub struct Inventory {
    pub obstacles: u32,
    pub turrets: u32,
}

#[derive(Component)]
pub struct SelectedBuildType(pub StructureType);

#[derive(Component)]
pub struct Hp {
    pub current: u32,
    pub max: u32,
}

impl Hp {
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn take_damage(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TurretDirection {
    North,
    East,
    South,
    West,
}

impl TurretDirection {
    pub fn to_vec3(&self) -> Vec3 {
        match self {
            TurretDirection::North => Vec3::Z,
            TurretDirection::East => Vec3::X,
            TurretDirection::South => -Vec3::Z,
            TurretDirection::West => -Vec3::X,
        }
    }

    pub fn from_quat(rotation: Quat) -> Self {
        let forward = rotation * Vec3::Z;
        let abs_x = forward.x.abs();
        let abs_z = forward.z.abs();

        if abs_x > abs_z {
            if forward.x > 0.0 {
                TurretDirection::East
            } else {
                TurretDirection::West
            }
        } else {
            if forward.z > 0.0 {
                TurretDirection::North
            } else {
                TurretDirection::South
            }
        }
    }

    pub fn to_quat(&self) -> Quat {
        match self {
            TurretDirection::North => Quat::IDENTITY,
            TurretDirection::East => Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
            TurretDirection::South => Quat::from_rotation_y(std::f32::consts::PI),
            TurretDirection::West => Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        }
    }
}

#[derive(Component)]
pub struct Turret {
    pub owner: Entity,
    pub direction: TurretDirection,
    pub last_shot: f32,
}

impl Turret {
    pub fn is_active(&self, current_time: f32) -> bool {
        current_time - self.last_shot >= TURRET_COOLDOWN
    }

    pub fn shot(&mut self, current_time: f32) {
        self.last_shot = current_time;
    }
}

#[derive(Component)]
pub struct Structure {
    pub ty: StructureType,
    pub collider_scale: f32,
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum StructureType {
    #[default]
    Obstacle,
    Wall,
    Turret,
}

#[derive(Component)]
pub struct Enemy;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                execute_movement,
                update_player_visibility,
                update_inventory,
                turret_shooting_system,
            ),
        );
    }
}

fn update_player_visibility(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Collectible>)>,
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
) {
    let players: Vec<(Entity, Vec3)> = player_query
        .iter()
        .map(|(e, t)| (e, t.translation))
        .collect();

    for (entity, pos) in &players {
        let mut visible = Vec::new();

        for (other_entity, other_pos) in &players {
            if *entity == *other_entity {
                continue;
            }

            if has_line_of_sight(*pos, *other_pos, &config, &grid) {
                visible.push(*other_entity);
            }
        }

        commands.entity(*entity).insert(PlayerStatus {
            visible_players: visible,
        });
    }
}

fn update_inventory(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Collectible>)>,
    mut inventory_query: Query<&mut Inventory, With<Player>>,
    collectible_query: Query<(Entity, &Transform, &Collectible), Without<Player>>,
) {
    let players: Vec<(Entity, Vec3)> = player_query
        .iter()
        .map(|(e, t)| (e, t.translation))
        .collect();
    let collectibles: Vec<(Entity, Vec3, crate::arena::CollectibleType)> = collectible_query
        .iter()
        .map(|(e, t, c)| (e, t.translation, c.ty))
        .collect();

    for (player_entity, player_pos) in &players {
        let mut collected_entities = Vec::new();
        let mut collected_obstacles = 0;
        let mut collected_turrets = 0;

        for (collectible_entity, collectible_pos, ty) in &collectibles {
            if player_pos.distance(*collectible_pos) < 2.0 {
                collected_entities.push(*collectible_entity);
                match ty {
                    crate::arena::CollectibleType::Obstacle => collected_obstacles += 1,
                    crate::arena::CollectibleType::Turret => collected_turrets += 1,
                }
            }
        }

        if !collected_entities.is_empty() {
            for entity in collected_entities {
                commands.entity(entity).despawn();
            }

            if let Ok(mut inventory) = inventory_query.get_mut(*player_entity) {
                inventory.obstacles += collected_obstacles;
                inventory.turrets += collected_turrets;
                info!(
                    "Collected! Obstacles: {}, Turrets: {}",
                    inventory.obstacles, inventory.turrets
                );
            }
        }
    }
}

fn aabb_overlaps(
    min1_x: f32,
    min1_z: f32,
    max1_x: f32,
    max1_z: f32,
    min2_x: f32,
    min2_z: f32,
    max2_x: f32,
    max2_z: f32,
) -> bool {
    min1_x < max2_x && max1_x > min2_x && min1_z < max2_z && max1_z > min2_z
}

fn execute_movement(
    time: Res<Time>,
    mut query: Query<
        (&mut Transform, &mut MovementController),
        (With<Player>, Without<Collectible>),
    >,
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
    structure_query: Query<&Structure>,
) {
    for (mut transform, mut controller) in query.iter_mut() {
        if controller.rotation_delta != 0.0 {
            transform.rotate_y(controller.rotation_delta);
        }

        let target_velocity = if controller.input_direction.length_squared() > 0.0 {
            let local_dir = transform.forward().as_vec3() * controller.input_direction.z
                + transform.right().as_vec3() * controller.input_direction.x;
            if local_dir.length_squared() > 0.0 {
                local_dir.normalize() * PLAYER_SPEED
            } else {
                Vec3::ZERO
            }
        } else {
            Vec3::ZERO
        };

        let dt = time.delta_secs();
        let vel_diff = target_velocity - controller.current_velocity;

        let mut velocity = if target_velocity.length() < controller.current_velocity.length() {
            controller.current_velocity + vel_diff * DECELERATION * dt
        } else {
            controller.current_velocity + vel_diff * ACCELERATION * dt
        };

        if velocity.length() > PLAYER_SPEED {
            velocity = velocity.normalize() * PLAYER_SPEED;
        }

        if (velocity.length() < 0.1) && (target_velocity == Vec3::ZERO) {
            velocity = Vec3::ZERO;
        }

        controller.current_velocity = velocity;

        if velocity.length_squared() > 0.0 {
            let pos_x = transform.translation.x;
            let pos_z = transform.translation.z;
            let half = PLAYER_SIZE * 0.5;
            let tile_size = config.tile_size;

            let dx = velocity.x * dt;
            let dz = velocity.z * dt;

            let player_min_x = pos_x - half;
            let player_max_x = pos_x + half;
            let player_min_z = pos_z - half;
            let player_max_z = pos_z + half;

            let next_min_x = pos_x + dx - half;
            let next_max_x = pos_x + dx + half;
            let next_min_z = pos_z + dz - half;
            let next_max_z = pos_z + dz + half;

            let mut move_x = true;
            let mut move_z = true;

            for (&key, &entity) in grid.occupants.iter() {
                let (tile_x, tile_z) = key;
                let Ok(structure) = structure_query.get(entity) else {
                    continue;
                };

                let half_size = tile_size * 0.5 * structure.collider_scale;
                let center_x = tile_x as f32 * tile_size + tile_size * 0.5;
                let center_z = tile_z as f32 * tile_size + tile_size * 0.5;

                let struct_min_x = center_x - half_size;
                let struct_max_x = center_x + half_size;
                let struct_min_z = center_z - half_size;
                let struct_max_z = center_z + half_size;

                if move_x
                    && aabb_overlaps(
                        next_min_x,
                        player_min_z,
                        next_max_x,
                        player_max_z,
                        struct_min_x,
                        struct_min_z,
                        struct_max_x,
                        struct_max_z,
                    )
                {
                    move_x = false;
                }

                if move_z
                    && aabb_overlaps(
                        player_min_x,
                        next_min_z,
                        player_max_x,
                        next_max_z,
                        struct_min_x,
                        struct_min_z,
                        struct_max_x,
                        struct_max_z,
                    )
                {
                    move_z = false;
                }
            }

            let mut final_dx = 0.0;
            let mut final_dz = 0.0;

            if move_x && move_z {
                final_dx = dx;
                final_dz = dz;
            } else if move_x {
                final_dx = dx;
                controller.current_velocity.z = 0.0;
            } else if move_z {
                final_dz = dz;
                controller.current_velocity.x = 0.0;
            } else {
                controller.current_velocity = Vec3::ZERO;
            }

            if final_dx != 0.0 || final_dz != 0.0 {
                transform.translation.x += final_dx;
                transform.translation.z += final_dz;
            }
        }
    }
}

fn turret_shooting_system(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    turret_query: Query<(Entity, &Transform, &Turret), Without<Enemy>>,
    mut enemy_query: Query<(Entity, &Transform, &mut Hp), With<Enemy>>,
) {
    let current_time = time.elapsed_secs();

    for (turret_entity, turret_transform, turret) in turret_query.iter() {
        if !turret.is_active(current_time) {
            continue;
        }

        let turret_pos = turret_transform.translation;
        let direction_vec = turret.direction.to_vec3();

        let mut closest_enemy: Option<Entity> = None;
        let mut closest_distance = f32::MAX;

        for (enemy_entity, enemy_transform, _) in enemy_query.iter() {
            let enemy_pos = enemy_transform.translation;
            let to_enemy = enemy_pos - turret_pos;
            let distance = to_enemy.length();

            if distance < 15.0 {
                let enemy_dir = to_enemy.normalize();
                let dot = enemy_dir.dot(direction_vec);

                if dot > 0.95 {
                    if distance < closest_distance {
                        closest_distance = distance;
                        closest_enemy = Some(enemy_entity);
                    }
                }
            }
        }

        if let Some(enemy_entity) = closest_enemy {
            commands.entity(turret_entity).insert(Turret {
                owner: turret.owner,
                direction: turret.direction,
                last_shot: current_time,
            });

            if let Ok((enemy_entity, enemy_transform, mut hp)) = enemy_query.get_mut(enemy_entity) {
                let old_hp = hp.current;
                hp.take_damage(TURRET_DAMAGE);
                if hp.is_alive() {
                    info!(
                        "Turret hit {:?}! HP: {}/{}",
                        enemy_entity, hp.current, hp.max
                    );
                } else {
                    info!("{:?} destroyed! Final HP: 0/{}", enemy_entity, hp.max);
                }
            }

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.2, 0.2, 0.2))),
                MeshMaterial3d(materials.add(Color::srgb(1.0, 1.0, 0.0))),
                Transform::from_translation(turret_pos + Vec3::Y * 2.0),
            ));
        }
    }
}
