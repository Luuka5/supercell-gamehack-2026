use crate::building::{Structure, StructureType};
use crate::pathfinding::NavGraph;
use areas::{Area, AreaMap};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rand::prelude::*;

pub mod areas;

pub struct ArenaDescription {
    pub layout: String,
    pub areas: Vec<Area>,
    pub player_spawn: Vec3,
    pub ai_spawn: Vec3,
    pub enemy_spawn: Vec3,
    pub resource_respawn_time: f32,
}

pub struct ArenaPlugin {
    description: ArenaDescription,
}

impl ArenaPlugin {
    pub fn new(description: ArenaDescription) -> Self {
        Self { description }
    }
}

#[derive(Resource)]
struct ArenaMapLayout(String);

#[derive(Resource)]
pub struct SpawnPoints {
    pub player: Vec3,
    pub ai: Vec3,
    pub enemy: Vec3,
}

#[derive(Resource)]
pub struct ResourceConfig {
    pub respawn_time: f32,
}

#[derive(Component)]
pub struct ResourceSpawner {
    pub ty: CollectibleType,
    pub timer: f32,
}

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        let lines: Vec<&str> = self.description.layout.trim().lines().collect();
        let height = lines.len() as u32;
        let width = lines.first().map(|l| l.len()).unwrap_or(0) as u32;

        app.insert_resource(ArenaConfig {
            width,
            height,
            tile_size: 4.0,
        })
        .insert_resource(ArenaMapLayout(self.description.layout.clone()))
        .insert_resource(AreaMap::new(self.description.areas.clone()))
        .insert_resource(SpawnPoints {
            player: self.description.player_spawn,
            ai: self.description.ai_spawn,
            enemy: self.description.enemy_spawn,
        })
        .insert_resource(ResourceConfig {
            respawn_time: self.description.resource_respawn_time,
        })
        .init_resource::<ArenaGrid>()
        .init_resource::<NavGraph>()
        .add_systems(Startup, spawn_arena)
        .add_systems(
            PostStartup,
            (generate_nav_nodes, calculate_area_connectivity),
        )
        .add_systems(Update, resource_respawn_system);
    }
}

fn resource_respawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner_query: Query<(&mut ResourceSpawner, &Transform)>,
    grid: Res<ArenaGrid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<ArenaConfig>,
) {
    for (mut spawner, transform) in spawner_query.iter_mut() {
        if spawner.timer > 0.0 {
            spawner.timer -= time.delta_secs();
            if spawner.timer <= 0.0 {
                // Respawn resource
                let x = ((transform.translation.x - config.tile_size * 0.5) / config.tile_size)
                    .floor() as u32;
                let y = ((transform.translation.z - config.tile_size * 0.5) / config.tile_size)
                    .floor() as u32;

                // Only spawn if tile is not occupied by a structure (collectibles don't block, but we don't want them inside walls)
                // Actually, collectibles are entities, but they are not in grid.occupants usually?
                // Let's check if there is already a collectible there?
                // For simplicity, just spawn it. The collection logic handles despawning.

                let collectible_mesh = meshes.add(Cuboid::new(0.5, 0.5, 0.5));
                let collectible_mat = if matches!(spawner.ty, CollectibleType::Turret) {
                    materials.add(Color::srgb(0.0, 0.0, 1.0))
                } else {
                    materials.add(Color::srgb(1.0, 1.0, 0.0))
                };

                commands.spawn((
                    Collectible { ty: spawner.ty },
                    Mesh3d(collectible_mesh),
                    MeshMaterial3d(collectible_mat),
                    Transform::from_translation(transform.translation + Vec3::Y * 0.5),
                ));
            }
        }
    }
}

pub fn calculate_area_connectivity(
    mut area_map: ResMut<AreaMap>,
    nav_graph: Res<NavGraph>,
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
) {
    let areas = area_map.areas.clone();
    let mut updates = Vec::new();

    for (i, area_a) in areas.iter().enumerate() {
        let mut neighbors = Vec::new();
        let mut visible = Vec::new();

        for (j, area_b) in areas.iter().enumerate() {
            if i == j {
                continue;
            }

            // Connectivity Check (Pathfinding)
            if let Some(_) = crate::pathfinding::find_path(area_a.center, area_b.center, &nav_graph)
            {
                // Simple check: if path exists, they are connected.
                // In reality, we might want to check if they are *adjacent* or if the path is short.
                // For now, let's assume if they are close enough (e.g. centers within 20 tiles) and reachable.
                let dist = ((area_a.center.0 as i32 - area_b.center.0 as i32).pow(2)
                    + (area_a.center.1 as i32 - area_b.center.1 as i32).pow(2))
                .abs();

                // This is a very rough heuristic. Ideally we check adjacency of boundaries.
                // But for now, let's just say everything is a neighbor if reachable.
                // Or better: check if we can raycast without hitting walls (Line of Sight)
                neighbors.push(area_b.id.clone());
            }

            // Visibility Check (Line of Sight)
            let start = Vec3::new(
                area_a.center.0 as f32 * config.tile_size + config.tile_size * 0.5,
                0.5,
                area_a.center.1 as f32 * config.tile_size + config.tile_size * 0.5,
            );
            let end = Vec3::new(
                area_b.center.0 as f32 * config.tile_size + config.tile_size * 0.5,
                0.5,
                area_b.center.1 as f32 * config.tile_size + config.tile_size * 0.5,
            );

            if crate::pathfinding::has_line_of_sight(start, end, &config, &grid) {
                visible.push(area_b.id.clone());
            }
        }
        updates.push((i, neighbors, visible));
    }

    for (index, neighbors, visible) in updates {
        area_map.areas[index].neighbors = neighbors;
        area_map.areas[index].visible_areas = visible;
    }

    info!("Area connectivity calculated.");
}

#[derive(Resource)]
pub struct ArenaConfig {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
}

#[derive(Resource, Default)]
pub struct ArenaGrid {
    pub tiles: HashMap<(u32, u32), Entity>,
    pub occupants: HashMap<(u32, u32), Entity>,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
}

#[derive(Component)]
pub struct Wall;

#[derive(Component)]
pub struct Obstacle;

#[derive(Component)]
pub struct SightBlocking;

#[derive(Component, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum CollectibleType {
    Obstacle,
    Turret,
}

#[derive(Component)]
pub struct Collectible {
    pub ty: CollectibleType,
}

fn spawn_arena(
    mut commands: Commands,
    config: Res<ArenaConfig>,
    layout: Res<ArenaMapLayout>,
    mut grid: ResMut<ArenaGrid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wall_height = 8.0;
    let floor_mesh = meshes.add(Cuboid::new(config.tile_size, 0.1, config.tile_size));
    let wall_mesh = meshes.add(Cuboid::new(config.tile_size, wall_height, config.tile_size));
    let obstacle_mesh = meshes.add(Cuboid::new(
        config.tile_size * 0.8,
        wall_height * 0.8,
        config.tile_size * 0.8,
    ));

    let floor_mat = materials.add(Color::srgb(0.3, 0.5, 0.3));
    let wall_mat = materials.add(Color::srgb(0.2, 0.2, 0.2));
    let obstacle_mat = materials.add(Color::srgb(0.6, 0.3, 0.3));

    let lines: Vec<&str> = layout.0.trim().lines().collect();

    for (y, line) in lines.iter().enumerate() {
        for (x, char) in line.chars().enumerate() {
            let x = x as u32;
            let y = y as u32;

            let position = Vec3::new(
                x as f32 * config.tile_size + config.tile_size * 0.5,
                0.0,
                y as f32 * config.tile_size + config.tile_size * 0.5,
            );

            let tile_entity = commands
                .spawn((
                    Tile { x, y },
                    Mesh3d(floor_mesh.clone()),
                    MeshMaterial3d(floor_mat.clone()),
                    Transform::from_translation(position),
                ))
                .id();

            grid.tiles.insert((x, y), tile_entity);

            match char {
                'X' => {
                    let wall_entity = commands
                        .spawn((
                            Wall,
                            SightBlocking,
                            Structure {
                                ty: StructureType::Wall,
                                collider_scale: 1.0,
                            },
                            Mesh3d(wall_mesh.clone()),
                            MeshMaterial3d(wall_mat.clone()),
                            Transform::from_translation(position + Vec3::Y * (wall_height / 2.0)),
                        ))
                        .id();
                    grid.occupants.insert((x, y), wall_entity);
                }
                'O' => {
                    let obstacle_entity = commands
                        .spawn((
                            Obstacle,
                            SightBlocking,
                            Structure {
                                ty: StructureType::Obstacle,
                                collider_scale: 1.0,
                            },
                            Mesh3d(obstacle_mesh.clone()),
                            MeshMaterial3d(obstacle_mat.clone()),
                            Transform::from_translation(position + Vec3::Y * (wall_height * 0.4)),
                        ))
                        .id();
                    grid.occupants.insert((x, y), obstacle_entity);
                }
                'T' | 'B' => {
                    // T = Turret Resource, B = Block (Obstacle) Resource
                    let collectible_type = if char == 'T' {
                        CollectibleType::Turret
                    } else {
                        CollectibleType::Obstacle
                    };

                    // Spawn Spawner
                    commands.spawn((
                        ResourceSpawner {
                            ty: collectible_type,
                            timer: 0.0, // Spawn immediately
                        },
                        Transform::from_translation(position),
                    ));
                }
                _ => {}
            }
        }
    }
}

pub fn generate_nav_nodes(
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
    mut nav_graph: ResMut<NavGraph>,
) {
    regenerate_nav_graph(&config, &grid, &mut nav_graph);
}

pub fn regenerate_nav_graph(config: &ArenaConfig, grid: &ArenaGrid, nav_graph: &mut NavGraph) {
    nav_graph.nodes.clear();
    let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    for ((x, y), _) in &grid.tiles {
        if grid.occupants.contains_key(&(*x, *y)) {
            continue;
        }

        let mut graph_neighbors = Vec::new();

        for (dx, dy) in directions {
            let nx = *x as i32 + dx;
            let ny = *y as i32 + dy;

            if nx >= 0 && nx < config.width as i32 && ny >= 0 && ny < config.height as i32 {
                let nx = nx as u32;
                let ny = ny as u32;

                if !grid.occupants.contains_key(&(nx, ny)) {
                    if let Some(&_neighbor_entity) = grid.tiles.get(&(nx, ny)) {
                        graph_neighbors.push((nx, ny));
                    }
                }
            }
        }

        nav_graph.nodes.insert((*x, *y), graph_neighbors);
    }

    info!("NavGraph generated with {} nodes.", nav_graph.nodes.len());
}
