use crate::arena::{ArenaConfig, ArenaGrid, Collectible, CollectibleType};
use crate::pathfinding::has_line_of_sight;
use bevy::prelude::*;

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct MovementController {
    pub input_direction: Vec3,
    pub rotation_delta: f32,
}

#[derive(Component)]
pub struct MainCamera {
    pub pitch: f32,
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

#[derive(Component, Default, Clone, Copy, PartialEq)]
pub enum BuildType {
    #[default]
    Obstacle,
    Turret,
}

#[derive(Component)]
pub struct SelectedBuildType(pub BuildType);

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
}

#[derive(Component)]
pub struct Structure {
    pub ty: StructureType,
    pub collider_scale: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum StructureType {
    Obstacle,
    Turret(TurretDirection),
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (execute_movement, update_player_visibility, update_inventory),
        );
    }
}

const PLAYER_SPEED: f32 = 20.0;

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
    let collectibles: Vec<(Entity, Vec3, CollectibleType)> = collectible_query
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
                    CollectibleType::Obstacle => collected_obstacles += 1,
                    CollectibleType::Turret => collected_turrets += 1,
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

fn execute_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &MovementController), (With<Player>, Without<Collectible>)>,
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
    structure_query: Query<&Structure>,
) {
    for (mut transform, controller) in query.iter_mut() {
        if controller.rotation_delta != 0.0 {
            transform.rotate_y(controller.rotation_delta);
        }

        if controller.input_direction.length_squared() > 0.0 {
            let local_dir = transform.forward().as_vec3() * controller.input_direction.z
                + transform.right().as_vec3() * controller.input_direction.x;

            if local_dir.length_squared() > 0.0 {
                let move_dir = local_dir.normalize();
                let speed = PLAYER_SPEED * time.delta_secs();

                let current_tile_x = (transform.translation.x / config.tile_size).round() as i32;
                let current_tile_y = (transform.translation.z / config.tile_size).round() as i32;

                let next_x = transform.translation.x + move_dir.x * speed;
                let next_y = transform.translation.z + move_dir.z * speed;

                let next_tile_x = (next_x / config.tile_size).round() as i32;
                let next_tile_y = (next_y / config.tile_size).round() as i32;

                let mut can_move_x = true;
                let mut can_move_y = true;

                if let Some(&occupant_entity) = grid
                    .occupants
                    .get(&(next_tile_x as u32, current_tile_y as u32))
                {
                    if let Ok(structure) = structure_query.get(occupant_entity) {
                        let tile_center_x = next_tile_x as f32 * config.tile_size;
                        let tile_center_y = current_tile_y as f32 * config.tile_size;
                        let collider_radius = structure.collider_scale * config.tile_size * 0.4;

                        if (next_x - tile_center_x).abs() > collider_radius {
                            can_move_x = false;
                        }
                    }
                }

                if let Some(&occupant_entity) = grid
                    .occupants
                    .get(&(current_tile_x as u32, next_tile_y as u32))
                {
                    if let Ok(structure) = structure_query.get(occupant_entity) {
                        let tile_center_x = current_tile_x as f32 * config.tile_size;
                        let tile_center_y = next_tile_y as f32 * config.tile_size;
                        let collider_radius = structure.collider_scale * config.tile_size * 0.4;

                        if (next_y - tile_center_y).abs() > collider_radius {
                            can_move_y = false;
                        }
                    }
                }

                let mut final_move = Vec3::ZERO;

                if can_move_x && can_move_y {
                    final_move = move_dir * speed;
                } else if can_move_x {
                    final_move = Vec3::new(move_dir.x * speed, 0.0, 0.0);
                } else if can_move_y {
                    final_move = Vec3::new(0.0, 0.0, move_dir.z * speed);
                }

                if final_move.length_squared() > 0.0 {
                    transform.translation += final_move;
                }
            }
        }
    }
}
