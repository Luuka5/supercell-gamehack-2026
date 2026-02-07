use crate::arena::areas::{AreaID, AreaMap};
use crate::arena::ArenaConfig;
use crate::combat::Hp;
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
        app.init_resource::<AreaMap>();
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
            &Inventory,
            &Transform,
            &mut TargetDestination,
        ),
        With<AiPlayer>,
    >,
    area_map: Res<AreaMap>,
    config: Res<ArenaConfig>,
) {
    for (entity, rule_set, status, hp, inventory, transform, mut target) in query.iter_mut() {
        // Sort rules by priority (descending)
        let mut sorted_rules = rule_set.0.rules.clone();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted_rules {
            if evaluate_condition(&rule.condition, status, hp, inventory) {
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
                        // TODO: Implement build action
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
