use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

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
        // Parse the layout to determine dimensions
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
    // Map (x, y) to the Tile entity at that location
    pub tiles: HashMap<(u32, u32), Entity>,
    // Map (x, y) to the Obstacle/Wall entity at that location (if any)
    pub occupants: HashMap<(u32, u32), Entity>,
}

#[derive(Resource, Default)]
pub struct NavGraph {
    pub nodes: HashMap<(u32, u32), Vec<(u32, u32)>>,
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

    // Iterate through the layout string
    // Note: The layout string is top-to-bottom (visual Y down), but our world coordinates usually have +Z as "down" or "south" in 3D.
    // Let's map layout row -> Z (y in grid coords) and layout col -> X (x in grid coords).
    // To match the previous loop behavior where (0,0) was bottom-left, we might need to reverse rows or just accept (0,0) is top-left of the string.
    // Let's treat the first line as Z=0 (or Z=height-1 if we want to flip it).
    // Standard convention: (0,0) is usually top-left in 2D grids from strings.
    // Let's map:
    // col -> x
    // row -> y (where row 0 is y=0)

    for (y, line) in lines.iter().enumerate() {
        for (x, char) in line.chars().enumerate() {
            let x = x as u32;
            let y = y as u32; // Using y from 0 to height-1

            let position = Vec3::new(
                x as f32 * config.tile_size,
                0.0,
                y as f32 * config.tile_size,
            );

            // Always spawn a floor tile
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
                '.' => {
                    // Empty floor, nothing to do
                }
                _ => {
                    // Unknown character, treat as floor
                }
            }
        }
    }
}

fn generate_nav_nodes(
    mut commands: Commands,
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
    mut nav_graph: ResMut<NavGraph>,
) {
    let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    for ((x, y), &tile_entity) in &grid.tiles {
        // If there is an occupant (Wall or Obstacle), this tile is not walkable
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

                // Check if neighbor is occupied
                if !grid.occupants.contains_key(&(nx, ny)) {
                    if let Some(&_neighbor_entity) = grid.tiles.get(&(nx, ny)) {
                        graph_neighbors.push((nx, ny));
                    }
                }
            }
        }

        // Add NavNode component to the tile entity
        nav_graph.nodes.insert((*x, *y), graph_neighbors);
    }

    info!("NavGraph generated with {} nodes.", nav_graph.nodes.len());
    // Debug specific nodes
    if let Some(n) = nav_graph.nodes.get(&(4, 2)) {
        info!("Node (4, 2) has {} neighbors: {:?}", n.len(), n);
    } else {
        warn!("Node (4, 2) not found in graph!");
    }
    if let Some(n) = nav_graph.nodes.get(&(35, 2)) {
        info!("Node (35, 2) has {} neighbors: {:?}", n.len(), n);
    } else {
        warn!("Node (35, 2) not found in graph!");
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    cost: u32,
    position: (u32, u32),
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.position.0.cmp(&other.position.0))
            .then_with(|| self.position.1.cmp(&other.position.1))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn find_path(start: (u32, u32), goal: (u32, u32), graph: &NavGraph) -> Option<Vec<(u32, u32)>> {
    let mut dist: HashMap<(u32, u32), u32> = HashMap::default();
    let mut heap = BinaryHeap::new();
    let mut came_from: HashMap<(u32, u32), (u32, u32)> = HashMap::default();

    dist.insert(start, 0);
    heap.push(State {
        cost: 0,
        position: start,
    });

    let mut visited_count = 0;

    while let Some(State { cost, position }) = heap.pop() {
        visited_count += 1;

        if position == goal {
            info!("Path found! Visited {} nodes.", visited_count);
            let mut path = Vec::new();
            let mut current = goal;
            while current != start {
                path.push(current);
                if let Some(&prev) = came_from.get(&current) {
                    current = prev;
                } else {
                    return None;
                }
            }
            path.push(start);
            path.reverse();
            return Some(path);
        }

        // Removed incorrect stale check
        // if cost > *dist.get(&position).unwrap_or(&u32::MAX) { continue; }

        if let Some(neighbors) = graph.nodes.get(&position) {
            for &neighbor in neighbors {
                let current_g = *dist.get(&position).unwrap();
                let new_cost = current_g + 1;

                let neighbor_dist = *dist.get(&neighbor).unwrap_or(&u32::MAX);

                if new_cost < neighbor_dist {
                    dist.insert(neighbor, new_cost);
                    let h = (neighbor.0 as i32 - goal.0 as i32).abs() as u32
                        + (neighbor.1 as i32 - goal.1 as i32).abs() as u32;
                    heap.push(State {
                        cost: new_cost + h,
                        position: neighbor,
                    });
                    came_from.insert(neighbor, position);
                }
            }
        } else {
            warn!("Node {:?} has no entry in graph!", position);
        }
    }

    info!(
        "Pathfinding failed. Visited {} nodes. Graph size: {}",
        visited_count,
        graph.nodes.len()
    );
    None
}
