use crate::pathfinding::NavGraph;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

pub struct ArenaPlugin {
    layout: String,
}

impl ArenaPlugin {
    pub fn new(layout: &str) -> Self {
        Self {
            layout: layout.to_string(),
        }
    }
}

#[derive(Resource)]
struct ArenaMapLayout(String);

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        let lines: Vec<&str> = self.layout.trim().lines().collect();
        let height = lines.len() as u32;
        let width = lines.first().map(|l| l.len()).unwrap_or(0) as u32;

        app.insert_resource(ArenaConfig {
            width,
            height,
            tile_size: 4.0,
        })
        .insert_resource(ArenaMapLayout(self.layout.clone()))
        .init_resource::<ArenaGrid>()
        .init_resource::<NavGraph>()
        .add_systems(Startup, spawn_arena)
        .add_systems(PostStartup, generate_nav_nodes);
    }
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
                x as f32 * config.tile_size,
                0.0,
                y as f32 * config.tile_size,
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
                            Mesh3d(obstacle_mesh.clone()),
                            MeshMaterial3d(obstacle_mat.clone()),
                            Transform::from_translation(position + Vec3::Y * (wall_height * 0.4)),
                        ))
                        .id();
                    grid.occupants.insert((x, y), obstacle_entity);
                }
                _ => {}
            }
        }
    }
}

fn generate_nav_nodes(
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
    mut nav_graph: ResMut<NavGraph>,
) {
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
