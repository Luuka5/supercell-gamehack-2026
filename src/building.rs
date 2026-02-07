use crate::ai::{AiPlayer, TargetDestination};
use crate::arena::{regenerate_nav_graph, ArenaConfig, ArenaGrid, Obstacle};
use crate::combat::{Turret, TurretDirection};
use crate::pathfinding::NavGraph;
use crate::player::Inventory;
use crate::user::{MainCamera, SelectedBuildType, User};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

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

pub struct BuildingPlugin;

impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_build_preview, handle_build_input));
    }
}

// Constants
const BUILD_MAX_DISTANCE: f32 = 15.0;

#[derive(Component)]
pub struct BuildGhost;

fn update_build_preview(
    mut commands: Commands,
    mut ghost_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<BuildGhost>,
    >,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    player_query: Query<&Transform, (With<User>, Without<BuildGhost>, Without<MainCamera>)>,
    config: Res<ArenaConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_query: Query<&SelectedBuildType, With<User>>,
) {
    let (camera, camera_transform): (&Camera, &GlobalTransform) =
        if let Some(c) = camera_query.iter().next() {
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
        let tile_x = (hit_point.x / config.tile_size).floor();
        let tile_z = (hit_point.z / config.tile_size).floor();

        let snapped_pos = Vec3::new(
            tile_x * config.tile_size + config.tile_size * 0.5,
            0.1, // Slightly above ground
            tile_z * config.tile_size + config.tile_size * 0.5,
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
        let Ok(selected) = selected_query.single() else {
            return;
        };

        if let Some((_, mut ghost_transform, mut visibility, mut ghost_material)) =
            ghost_query.iter_mut().next()
        {
            ghost_transform.translation = snapped_pos;
            *visibility = if in_range {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };

            match selected.0 {
                StructureType::Obstacle => {
                    ghost_material.0 = materials.add(Color::srgba(0.6, 0.3, 0.3, 0.5));
                }
                StructureType::Turret => {
                    ghost_material.0 = materials.add(Color::srgba(0.0, 0.5, 1.0, 0.5));
                }
                StructureType::Wall => {}
            }
        } else {
            let (mesh, color) = match selected.0 {
                StructureType::Obstacle => (
                    meshes.add(Cuboid::new(
                        config.tile_size * 0.8,
                        6.4,
                        config.tile_size * 0.8,
                    )),
                    Color::srgba(0.6, 0.3, 0.3, 0.5),
                ),
                StructureType::Turret => (
                    meshes.add(Cylinder::new(1.5, 3.0)),
                    Color::srgba(0.0, 0.5, 1.0, 0.5),
                ),
                StructureType::Wall => (
                    meshes.add(Cuboid::new(config.tile_size, 8.0, config.tile_size)),
                    Color::srgba(0.2, 0.2, 0.2, 1.0),
                ),
            };

            commands.spawn((
                BuildGhost,
                Mesh3d(mesh),
                MeshMaterial3d(materials.add(color)),
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
    time: Res<Time>,
    mut grid: ResMut<ArenaGrid>,
    mut nav_graph: ResMut<NavGraph>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ai_query: Query<&mut TargetDestination, With<AiPlayer>>,
    selected_query: Query<&SelectedBuildType, With<User>>,
    player_query: Query<(Entity, &Transform), With<User>>,
    mut inventory_query: Query<&mut Inventory, With<User>>,
) {
    if let Some((transform, visibility)) = ghost_query.iter().next() {
        if visibility == Visibility::Hidden {
            return;
        }

        let pos = transform.translation;
        let tile_x = ((pos.x - config.tile_size * 0.5) / config.tile_size).floor() as u32;
        let tile_y = ((pos.z - config.tile_size * 0.5) / config.tile_size).floor() as u32;

        let mut graph_dirty = false;

        // Build (Right Click)
        if mouse_btn.just_pressed(MouseButton::Right) {
            if grid.occupants.contains_key(&(tile_x, tile_y)) {
                info!("Cannot build here: Occupied");
                return;
            }

            let Ok(selected) = selected_query.single() else {
                return;
            };

            let (player_entity, player_transform) = if let Ok(p) = player_query.single() {
                p
            } else {
                return;
            };

            let Ok(mut inventory) = inventory_query.single_mut() else {
                return;
            };

            let players_at_target = player_query.iter().any(|(_, p_transform)| {
                let p_tile_x = ((p_transform.translation.x - config.tile_size * 0.5)
                    / config.tile_size)
                    .floor() as u32;
                let p_tile_y = ((p_transform.translation.z - config.tile_size * 0.5)
                    / config.tile_size)
                    .floor() as u32;
                p_tile_x == tile_x && p_tile_y == tile_y
            });

            if players_at_target {
                info!("Cannot build here: Player is on this tile");
                return;
            }

            match selected.0 {
                StructureType::Obstacle => {
                    if inventory.obstacles == 0 {
                        info!("No obstacles left!");
                        return;
                    }

                    let obstacle_mesh = meshes.add(Cuboid::new(
                        config.tile_size * 0.8,
                        8.0 * 0.8,
                        config.tile_size * 0.8,
                    ));
                    let obstacle_mat = materials.add(Color::srgb(0.6, 0.3, 0.3));

                    let obstacle_entity = commands
                        .spawn((
                            Obstacle,
                            Structure {
                                ty: StructureType::Obstacle,
                                collider_scale: 1.0,
                            },
                            Mesh3d(obstacle_mesh),
                            MeshMaterial3d(obstacle_mat),
                            Transform::from_translation(pos + Vec3::Y * (8.0 * 0.4)),
                        ))
                        .id();

                    grid.occupants.insert((tile_x, tile_y), obstacle_entity);
                    inventory.obstacles -= 1;
                    info!("Built obstacle at ({}, {})", tile_x, tile_y);
                    graph_dirty = true;
                }
                StructureType::Turret => {
                    if inventory.turrets == 0 {
                        info!("No turrets left!");
                        return;
                    }

                    let player_transform = if let Ok(t) = player_query.single().map(|(_, t)| t) {
                        t
                    } else {
                        return;
                    };

                    let forward = player_transform.forward();
                    let abs_x = forward.x.abs();
                    let abs_z = forward.z.abs();
                    let actual_direction = if abs_x > abs_z {
                        if forward.x > 0.0 {
                            TurretDirection::East
                        } else {
                            TurretDirection::West
                        }
                    } else {
                        if forward.z > 0.0 {
                            TurretDirection::South
                        } else {
                            TurretDirection::North
                        }
                    };

                    let turret_mesh = meshes.add(Cylinder::new(1.5, 3.0));
                    let turret_mat = materials.add(Color::srgb(0.0, 0.5, 1.0));
                    let barrel_mesh = meshes.add(Cuboid::new(0.5, 0.5, 2.0));
                    let barrel_mat = materials.add(Color::srgb(0.2, 0.2, 0.8));

                    let rotation = actual_direction.to_quat();
                    // Barrel offset should be in local space (always forward relative to turret)
                    // Since North is -Z, forward is -Z.
                    let barrel_offset = -Vec3::Z * 2.5;

                    let turret_entity = commands
                        .spawn((
                            Obstacle,
                            Structure {
                                ty: StructureType::Turret,
                                collider_scale: 0.7,
                            },
                            Turret {
                                owner: player_entity,
                                direction: actual_direction,
                                last_shot: time.elapsed_secs() - 4.0,
                            },
                            Mesh3d(turret_mesh),
                            MeshMaterial3d(turret_mat),
                            Transform::from_translation(pos + Vec3::Y * 1.5)
                                .with_rotation(rotation),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Mesh3d(barrel_mesh),
                                MeshMaterial3d(barrel_mat),
                                Transform::from_translation(barrel_offset + Vec3::Y * 0.5),
                            ));
                        })
                        .id();

                    grid.occupants.insert((tile_x, tile_y), turret_entity);
                    inventory.turrets -= 1;
                    info!(
                        "Built turret at ({}, {}) facing {:?}",
                        tile_x, tile_y, actual_direction
                    );
                }
                StructureType::Wall => {}
            }
        }

        // Destroy (Left Click)
        if mouse_btn.just_pressed(MouseButton::Left) {
            if let Some(&occupant_entity) = grid.occupants.get(&(tile_x, tile_y)) {
                commands.entity(occupant_entity).despawn();
                grid.occupants.remove(&(tile_x, tile_y));
                info!("Destroyed structure at ({}, {})", tile_x, tile_y);
                graph_dirty = true;
            } else {
                info!("Nothing to destroy at ({}, {})", tile_x, tile_y);
            }
        }

        if graph_dirty {
            regenerate_nav_graph(&config, &grid, &mut nav_graph);

            for mut target in ai_query.iter_mut() {
                target.set_changed();
            }
        }
    }
}
