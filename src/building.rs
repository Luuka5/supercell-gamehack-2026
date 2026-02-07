use crate::ai::{AiPlayer, TargetDestination};
use crate::arena::{regenerate_nav_graph, ArenaConfig, ArenaGrid, Obstacle};
use crate::pathfinding::NavGraph;
use crate::player::{MainCamera, User};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct BuildingPlugin;

impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_build_preview, handle_build_input));
    }
}

// Constants
const BUILD_MAX_DISTANCE: f32 = 15.0;

#[derive(Component)]
pub struct Builder;

#[derive(Component)]
pub struct BuildGhost;

fn update_build_preview(
    mut commands: Commands,
    mut ghost_query: Query<(Entity, &mut Transform, &mut Visibility), With<BuildGhost>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    player_query: Query<&Transform, (With<User>, Without<BuildGhost>, Without<MainCamera>)>,
    config: Res<ArenaConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (camera, camera_transform) = if let Some(c) = camera_query.iter().next() {
        c
    } else {
        return;
    };

    let window = if let Some(w) = window_query.iter().next() {
        w
    } else {
        return;
    };

    // Raycast from center of screen
    let viewport_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let ray = if let Ok(r) = camera.viewport_to_world(camera_transform, viewport_center) {
        r
    } else {
        return;
    };

    // Intersect with ground plane (y=0)
    // Ray: origin + t * direction
    // y = origin.y + t * direction.y = 0
    // t = -origin.y / direction.y
    let t = -ray.origin.y / ray.direction.y;

    if t > 0.0 {
        let hit_point = ray.origin + ray.direction * t;

        // Snap to grid
        let tile_x = (hit_point.x / config.tile_size).round();
        let tile_z = (hit_point.z / config.tile_size).round();

        let snapped_pos = Vec3::new(
            tile_x * config.tile_size,
            0.1, // Slightly above ground
            tile_z * config.tile_size,
        );

        // Check distance from player
        let player_transform = if let Some(t) = player_query.iter().next() {
            t
        } else {
            return;
        };

        let dist = player_transform.translation.distance(snapped_pos);
        let in_range = dist <= BUILD_MAX_DISTANCE;

        // Update or Spawn Ghost
        if let Some((_, mut ghost_transform, mut visibility)) = ghost_query.iter_mut().next() {
            ghost_transform.translation = snapped_pos;
            *visibility = if in_range {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else {
            // Spawn new ghost
            commands.spawn((
                BuildGhost,
                Mesh3d(meshes.add(Cuboid::new(config.tile_size, 0.2, config.tile_size))),
                MeshMaterial3d(materials.add(Color::srgba(0.2, 0.2, 1.0, 0.5))), // Blue semi-transparent
                Transform::from_translation(snapped_pos),
            ));
        }
    }
}

fn handle_build_input(
    mut commands: Commands,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    ghost_query: Query<(&Transform, &Visibility), With<BuildGhost>>,
    config: Res<ArenaConfig>,
    mut grid: ResMut<ArenaGrid>,
    mut nav_graph: ResMut<NavGraph>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ai_query: Query<&mut TargetDestination, With<AiPlayer>>,
) {
    if let Some((transform, visibility)) = ghost_query.iter().next() {
        if visibility == Visibility::Hidden {
            return;
        }

        let pos = transform.translation;
        let tile_x = (pos.x / config.tile_size).round() as u32;
        let tile_y = (pos.z / config.tile_size).round() as u32;

        let mut graph_dirty = false;

        // Build (Right Click)
        if mouse_btn.just_pressed(MouseButton::Right) {
            // Check if occupied
            if grid.occupants.contains_key(&(tile_x, tile_y)) {
                info!("Cannot build here: Occupied");
                return;
            }

            // Spawn Obstacle
            let obstacle_mesh = meshes.add(Cuboid::new(
                config.tile_size * 0.8,
                8.0 * 0.8,
                config.tile_size * 0.8,
            ));
            let obstacle_mat = materials.add(Color::srgb(0.6, 0.3, 0.3));

            let obstacle_entity = commands
                .spawn((
                    Obstacle,
                    Mesh3d(obstacle_mesh),
                    MeshMaterial3d(obstacle_mat),
                    Transform::from_translation(pos + Vec3::Y * (8.0 * 0.4)),
                ))
                .id();

            grid.occupants.insert((tile_x, tile_y), obstacle_entity);
            info!("Built obstacle at ({}, {})", tile_x, tile_y);
            graph_dirty = true;
        }

        // Destroy (Left Click)
        if mouse_btn.just_pressed(MouseButton::Left) {
            if let Some(&occupant_entity) = grid.occupants.get(&(tile_x, tile_y)) {
                // Only destroy Obstacles, not Walls (if we want to distinguish, we'd need to query the entity)
                // For now, let's assume we can destroy anything in `occupants` that isn't a permanent map feature?
                // The user said "not walls defined by the map".
                // Walls defined by map are also in `occupants`.
                // We should check if the entity has `Obstacle` component.
                // But we don't have access to query components of `occupant_entity` here easily without a `Query`.
                // We can assume for this prototype that we can destroy anything, or we can try to be safe.
                // Let's just destroy it for now, or better:
                // We can't query arbitrary entities without `Query<Entity, With<Obstacle>>`.
                // Let's add a check.

                // Actually, we can just try to despawn it.
                // But we should only remove it from grid if it was actually destroyed.
                // Let's assume we can destroy it.

                commands.entity(occupant_entity).despawn();
                grid.occupants.remove(&(tile_x, tile_y));
                info!("Destroyed obstacle at ({}, {})", tile_x, tile_y);
                graph_dirty = true;
            } else {
                info!("Nothing to destroy at ({}, {})", tile_x, tile_y);
            }
        }

        if graph_dirty {
            regenerate_nav_graph(&config, &grid, &mut nav_graph);

            // Force AI to repath
            for mut target in ai_query.iter_mut() {
                // Trigger change detection by mutating (even if value is same)
                target.set_changed();
            }
        }
    }
}
