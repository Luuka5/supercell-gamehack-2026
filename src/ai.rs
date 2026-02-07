use crate::arena::ArenaConfig;
use crate::pathfinding::{find_path, NavGraph};
use crate::player::MovementController;
use bevy::prelude::*;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (pathfinding_system, path_following_system));
    }
}

#[derive(Component)]
pub struct AiPlayer;

#[derive(Component)]
pub struct Enemy;

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
            info!(
                "Path found for AI {:?}: {} steps from ({}, {}) to ({}, {})",
                entity,
                path.len(),
                start_x,
                start_y,
                target.x,
                target.y
            );
            commands.entity(entity).insert(PathFollower {
                path,
                current_index: 0,
            });
        } else {
            warn!(
                "No path found for AI {:?} from ({}, {}) to ({}, {})!",
                entity, start_x, start_y, target.x, target.y
            );
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
            info!(
                "AI {:?} reached node {:?}. Next: {:?}",
                entity, target_node, follower.current_index
            );
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
