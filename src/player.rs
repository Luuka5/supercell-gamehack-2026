use crate::arena::areas::{AreaID, AreaMap};
use crate::arena::{ArenaConfig, ArenaGrid, Collectible};
use crate::building::Structure;
use crate::logging::{GameEvent, MatchLog};
use crate::pathfinding::{find_path, has_line_of_sight, NavGraph};
use bevy::prelude::*;
use std::collections::HashMap;

pub const PLAYER_SPEED: f32 = 20.0;
pub const ACCELERATION: f32 = 10.0;
pub const DECELERATION: f32 = 30.0;

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
    pub nearest_enemy_position: Option<Vec3>,
    pub nearest_enemy_dist: f32,
    pub current_area_id: Option<AreaID>,
    pub area_distances: HashMap<AreaID, u32>,
    pub visible_areas_from_self: Vec<AreaID>,
}

#[derive(Component, Default)]
pub struct Inventory {
    pub obstacles: u32,
    pub turrets: u32,
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

fn update_player_visibility(
    mut commands: Commands,
    mut player_query: Query<
        (Entity, &Transform, &mut PlayerStatus),
        (With<Player>, Without<Collectible>),
    >,
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
    area_map: Option<Res<AreaMap>>,
    nav_graph: Res<NavGraph>,
    mut match_log: ResMut<MatchLog>,
    time: Res<Time>,
) {
    let players: Vec<(Entity, Vec3)> = player_query
        .iter()
        .map(|(e, t, _)| (e, t.translation))
        .collect();

    for (entity, transform, mut status) in player_query.iter_mut() {
        let pos = transform.translation;
        let mut visible = Vec::new();
        let mut nearest_enemy_pos = None;
        let mut nearest_dist = f32::MAX;

        for (other_entity, other_pos) in &players {
            if entity == *other_entity {
                continue;
            }

            if has_line_of_sight(pos, *other_pos, &config, &grid) {
                visible.push(*other_entity);
                let dist = pos.distance(*other_pos);
                if dist < nearest_dist {
                    nearest_dist = dist;
                    nearest_enemy_pos = Some(*other_pos);
                }
            }
        }

        let mut area_distances = HashMap::default();
        let mut visible_areas_from_self = Vec::new();

        let current_area = if let Some(map) = &area_map {
            let tile_x = ((pos.x - config.tile_size * 0.5) / config.tile_size).floor() as u32;
            let tile_y = ((pos.z - config.tile_size * 0.5) / config.tile_size).floor() as u32;

            // Calculate distances to all areas
            for area in &map.areas {
                if let Some(path) = find_path((tile_x, tile_y), area.center, &nav_graph) {
                    area_distances.insert(area.id.clone(), path.len() as u32);
                }

                // Check visibility to area center
                let area_center_world = Vec3::new(
                    area.center.0 as f32 * config.tile_size + config.tile_size * 0.5,
                    0.5,
                    area.center.1 as f32 * config.tile_size + config.tile_size * 0.5,
                );

                if has_line_of_sight(pos, area_center_world, &config, &grid) {
                    visible_areas_from_self.push(area.id.clone());
                }
            }

            let new_area_id = map.get_area_id(tile_x, tile_y);

            // Log area change
            if status.current_area_id.as_ref() != Some(&new_area_id) {
                match_log.add(GameEvent::AreaEntered {
                    entity,
                    area_id: new_area_id.clone(),
                    time: time.elapsed_secs(),
                });
            }

            Some(new_area_id)
        } else {
            None
        };

        status.visible_players = visible;
        status.nearest_enemy_position = nearest_enemy_pos;
        status.nearest_enemy_dist = nearest_dist;
        status.current_area_id = current_area;
        status.area_distances = area_distances;
        status.visible_areas_from_self = visible_areas_from_self;
    }
}

fn update_inventory(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Collectible>)>,
    mut inventory_query: Query<&mut Inventory, With<Player>>,
    collectible_query: Query<(Entity, &Transform, &Collectible), Without<Player>>,
    mut match_log: ResMut<MatchLog>,
    time: Res<Time>,
    config: Res<ArenaConfig>,
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

                let tile_x = ((collectible_pos.x - config.tile_size * 0.5) / config.tile_size)
                    .floor() as u32;
                let tile_y = ((collectible_pos.z - config.tile_size * 0.5) / config.tile_size)
                    .floor() as u32;

                match_log.add(GameEvent::ItemCollected {
                    entity: *player_entity,
                    item_type: *ty,
                    location: (tile_x, tile_y),
                    time: time.elapsed_secs(),
                });

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
