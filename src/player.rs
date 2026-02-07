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
                transform.translation += move_dir * PLAYER_SPEED * time.delta_secs();
            }
        }
    }
}
