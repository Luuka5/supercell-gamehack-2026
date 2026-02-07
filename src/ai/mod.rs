use crate::arena::areas::AreaMap;
use crate::arena::{ArenaConfig, ArenaGrid, Obstacle};
use crate::building::{Structure, StructureType};
use crate::combat::{Hp, Turret, TurretDirection};
use crate::pathfinding::{find_path, NavGraph};
use crate::player::{Inventory, MovementController, PlayerStatus};
use bevy::prelude::*;

pub mod rules;

use rules::{Action, Condition, Rule, RuleSet};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                pathfinding_system,
                path_following_system,
                rule_evaluation_system,
            ),
        );
        // AreaMap is now initialized by ArenaPlugin
        // app.init_resource::<AreaMap>();
    }
}

#[derive(Component)]
pub struct AiPlayer;

#[derive(Component)]
pub struct TargetDestination {
    pub x: u32,
    pub y: u32,
}

#[derive(Component, Default)]
pub struct PathFollower {
    pub path: Vec<(u32, u32)>,
    pub current_index: usize,
}

#[derive(Component, Default)]
pub struct AiRuleSet(pub RuleSet);

fn evaluate_condition(
    condition: &Condition,
    status: &PlayerStatus,
    hp: &Hp,
    inventory: &Inventory,
) -> bool {
    match condition {
        Condition::True => true,
        Condition::IsEnemyVisible => !status.visible_players.is_empty(),
        Condition::IsHealthLow { threshold } => hp.current <= *threshold,
        Condition::InArea(area_id) => status.current_area_id.as_ref() == Some(area_id),
        Condition::HasItem { item, count } => match item.as_str() {
            "obstacle" => inventory.obstacles >= *count,
            "turret" => inventory.turrets >= *count,
            _ => false,
        },
        Condition::IsUnderAttack => false, // TODO: Implement attack detection
        Condition::And(conditions) => conditions
            .iter()
            .all(|c| evaluate_condition(c, status, hp, inventory)),
        Condition::Or(conditions) => conditions
            .iter()
            .any(|c| evaluate_condition(c, status, hp, inventory)),
        Condition::Not(condition) => !evaluate_condition(condition, status, hp, inventory),
    }
}

fn rule_evaluation_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &AiRuleSet,
            &PlayerStatus,
            &Hp,
            &mut Inventory,
            &Transform,
            &mut TargetDestination,
        ),
        With<AiPlayer>,
    >,
    area_map: Res<AreaMap>,
    config: Res<ArenaConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut grid: ResMut<ArenaGrid>,
    mut nav_graph: ResMut<NavGraph>,
) {
    for (entity, rule_set, status, hp, mut inventory, transform, mut target) in query.iter_mut() {
        // Sort rules by priority (descending)
        let mut sorted_rules = rule_set.0.rules.clone();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted_rules {
            if evaluate_condition(&rule.condition, status, hp, &inventory) {
                // info!("AI {:?} executing rule: {}", entity, rule.name);
                match &rule.action {
                    Action::MoveToArea(area_id) => {
                        if let Some((x, y)) = area_map.get_center(area_id.clone()) {
                            if target.x != x || target.y != y {
                                target.x = x;
                                target.y = y;
                                // info!("AI {:?} moving to area {:?}", entity, area_id);
                            }
                        }
                    }
                    Action::ChaseEnemy => {
                        if let Some(enemy_pos) = status.nearest_enemy_position {
                            let x = ((enemy_pos.x - config.tile_size * 0.5) / config.tile_size)
                                .floor() as u32;
                            let y = ((enemy_pos.z - config.tile_size * 0.5) / config.tile_size)
                                .floor() as u32;

                            if target.x != x || target.y != y {
                                target.x = x;
                                target.y = y;
                                // info!("AI {:?} chasing enemy at ({}, {})", entity, x, y);
                            }
                        }
                    }
                    Action::Flee => {
                        // Simple flee: Run to opposite corner of nearest enemy
                        if let Some(enemy_pos) = status.nearest_enemy_position {
                            let my_x = ((transform.translation.x - config.tile_size * 0.5)
                                / config.tile_size)
                                .floor() as u32;
                            let my_y = ((transform.translation.z - config.tile_size * 0.5)
                                / config.tile_size)
                                .floor() as u32;

                            let enemy_x = ((enemy_pos.x - config.tile_size * 0.5)
                                / config.tile_size)
                                .floor() as u32;
                            let enemy_y = ((enemy_pos.z - config.tile_size * 0.5)
                                / config.tile_size)
                                .floor() as u32;

                            // Vector away from enemy
                            let dx = my_x as i32 - enemy_x as i32;
                            let dy = my_y as i32 - enemy_y as i32;

                            let flee_x = (my_x as i32 + dx).clamp(0, config.width as i32) as u32;
                            let flee_y = (my_y as i32 + dy).clamp(0, config.height as i32) as u32;

                            if target.x != flee_x || target.y != flee_y {
                                target.x = flee_x;
                                target.y = flee_y;
                                // info!("AI {:?} fleeing to ({}, {})", entity, flee_x, flee_y);
                            }
                        }
                    }
                    Action::Build {
                        structure,
                        direction,
                    } => {
                        let tile_x = ((transform.translation.x - config.tile_size * 0.5)
                            / config.tile_size)
                            .floor() as u32;
                        let tile_y = ((transform.translation.z - config.tile_size * 0.5)
                            / config.tile_size)
                            .floor() as u32;

                        // Check if tile is occupied
                        if !grid.occupants.contains_key(&(tile_x, tile_y)) {
                            let position = Vec3::new(
                                tile_x as f32 * config.tile_size + config.tile_size * 0.5,
                                0.0,
                                tile_y as f32 * config.tile_size + config.tile_size * 0.5,
                            );

                            match structure {
                                StructureType::Obstacle => {
                                    if inventory.obstacles > 0 {
                                        inventory.obstacles -= 1;
                                        let obstacle_entity = commands
                                            .spawn((
                                                Obstacle,
                                                Structure {
                                                    ty: StructureType::Obstacle,
                                                    collider_scale: 1.0,
                                                },
                                                Mesh3d(meshes.add(Cuboid::new(
                                                    config.tile_size * 0.8,
                                                    8.0 * 0.8,
                                                    config.tile_size * 0.8,
                                                ))),
                                                MeshMaterial3d(
                                                    materials.add(Color::srgb(0.6, 0.3, 0.3)),
                                                ),
                                                Transform::from_translation(
                                                    position + Vec3::Y * (8.0 * 0.4),
                                                ),
                                            ))
                                            .id();
                                        grid.occupants.insert((tile_x, tile_y), obstacle_entity);
                                        crate::arena::regenerate_nav_graph(
                                            &config,
                                            &grid,
                                            &mut nav_graph,
                                        );
                                        info!("AI Built Obstacle at ({}, {})", tile_x, tile_y);
                                    }
                                }
                                StructureType::Turret => {
                                    if inventory.turrets > 0 {
                                        inventory.turrets -= 1;

                                        let turret_dir = if let Some(dir) = direction {
                                            *dir
                                        } else {
                                            // Face enemy if possible, else random or South
                                            if let Some(enemy_pos) = status.nearest_enemy_position {
                                                let to_enemy = enemy_pos - transform.translation;
                                                // Determine cardinal direction
                                                if to_enemy.x.abs() > to_enemy.z.abs() {
                                                    if to_enemy.x > 0.0 {
                                                        TurretDirection::East
                                                    } else {
                                                        TurretDirection::West
                                                    }
                                                } else {
                                                    if to_enemy.z > 0.0 {
                                                        TurretDirection::South
                                                    } else {
                                                        TurretDirection::North
                                                    }
                                                }
                                            } else {
                                                TurretDirection::South
                                            }
                                        };

                                        let rotation = turret_dir.to_quat();
                                        let barrel_offset = -Vec3::Z * 2.5;

                                        let turret_entity = commands
                                            .spawn((
                                                Turret {
                                                    owner: entity,
                                                    direction: turret_dir,
                                                    last_shot: 0.0,
                                                },
                                                Structure {
                                                    ty: StructureType::Turret,
                                                    collider_scale: 0.5,
                                                },
                                                Mesh3d(meshes.add(Cuboid::new(1.0, 2.0, 1.0))),
                                                MeshMaterial3d(
                                                    materials.add(Color::srgb(0.2, 0.2, 0.2)),
                                                ),
                                                Transform::from_translation(
                                                    position + Vec3::Y * 1.0,
                                                )
                                                .with_rotation(rotation),
                                            ))
                                            .with_children(|parent| {
                                                parent.spawn((
                                                    Mesh3d(meshes.add(Cuboid::new(0.4, 0.4, 3.0))),
                                                    MeshMaterial3d(
                                                        materials.add(Color::srgb(0.1, 0.1, 0.1)),
                                                    ),
                                                    Transform::from_translation(barrel_offset),
                                                ));
                                            })
                                            .id();

                                        grid.occupants.insert((tile_x, tile_y), turret_entity);
                                        crate::arena::regenerate_nav_graph(
                                            &config,
                                            &grid,
                                            &mut nav_graph,
                                        );
                                        info!(
                                            "AI Built Turret at ({}, {}) facing {:?}",
                                            tile_x, tile_y, turret_dir
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Action::Idle => {
                        // Do nothing
                    }
                }
                break; // Execute only the highest priority rule
            }
        }
    }
}

fn pathfinding_system(
    mut commands: Commands,
    mut query: Query<
        (Entity, &Transform, &TargetDestination),
        (With<AiPlayer>, Changed<TargetDestination>),
    >,
    nav_graph: Res<NavGraph>,
    config: Res<ArenaConfig>,
) {
    for (entity, transform, target) in query.iter_mut() {
        let start_x =
            ((transform.translation.x - config.tile_size * 0.5) / config.tile_size).floor() as u32;
        let start_y =
            ((transform.translation.z - config.tile_size * 0.5) / config.tile_size).floor() as u32;

        if let Some(path) = find_path((start_x, start_y), (target.x, target.y), &nav_graph) {
            /*
            info!(
                "Path found for AI {:?}: {} steps from ({}, {}) to ({}, {})",
                entity,
                path.len(),
                start_x,
                start_y,
                target.x,
                target.y
            );
            */
            commands.entity(entity).insert(PathFollower {
                path,
                current_index: 0,
            });
        } else {
            /*
            warn!(
                "No path found for AI {:?} from ({}, {}) to ({}, {})!",
                entity, start_x, start_y, target.x, target.y
            );
            */
        }
    }
}

fn path_following_system(
    mut query: Query<
        (
            Entity,
            &Transform,
            &mut PathFollower,
            &mut MovementController,
        ),
        With<AiPlayer>,
    >,
    config: Res<ArenaConfig>,
) {
    for (entity, transform, mut follower, mut controller) in query.iter_mut() {
        if follower.current_index >= follower.path.len() {
            controller.input_direction = Vec3::ZERO;
            continue;
        }

        let target_node = follower.path[follower.current_index];
        let target_pos = Vec3::new(
            target_node.0 as f32 * config.tile_size + config.tile_size * 0.5,
            transform.translation.y,
            target_node.1 as f32 * config.tile_size + config.tile_size * 0.5,
        );

        let direction = target_pos - transform.translation;
        let distance = direction.length();

        if distance < 0.5 {
            follower.current_index += 1;
            /*
            info!(
                "AI {:?} reached node {:?}. Next: {:?}",
                entity, target_node, follower.current_index
            );
            */
        } else {
            let forward = direction.normalize();
            let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();

            if flat_forward != Vec3::ZERO {
                let local_direction = transform.rotation.inverse() * direction;
                let move_dir = local_direction.normalize();

                controller.input_direction = Vec3::new(move_dir.x, 0.0, -move_dir.z);
                controller.rotation_delta = -move_dir.x * 0.1;
            }
        }
    }
}
