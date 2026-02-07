use crate::arena::{ArenaConfig, ArenaGrid};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Resource, Default)]
pub struct NavGraph {
    pub nodes: HashMap<(u32, u32), Vec<(u32, u32)>>,
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

pub fn regenerate_nav_graph(config: &ArenaConfig, grid: &ArenaGrid, nav_graph: &mut NavGraph) {
    nav_graph.nodes.clear();
    // Include diagonals: (dx, dy, cost_multiplier)
    // Cardinal: 1.0, Diagonal: 1.414 (sqrt(2))
    // We'll store neighbors as just coordinates for now, but we need to handle cost in find_path.
    // Wait, NavGraph currently stores Vec<(u32, u32)>. It doesn't store edge weights.
    // We need to update NavGraph to store weights or calculate them on the fly.
    // Let's update find_path to calculate distance based on coordinates.

    let directions = [
        (0, 1),
        (0, -1),
        (1, 0),
        (-1, 0), // Cardinal
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1), // Diagonal
    ];

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
                    // For diagonals, check if we are cutting a corner.
                    // If moving (1, 1), check (1, 0) and (0, 1). If both are blocked, we can't move.
                    // If one is blocked, it's usually okay in games, but strictly speaking might clip.
                    // Let's prevent corner cutting if BOTH adjacent cardinals are blocked?
                    // Or even if ONE is blocked (strict).
                    // Let's go with strict: if either cardinal neighbor is blocked, diagonal is blocked.

                    let mut blocked = false;
                    if dx.abs() == 1 && dy.abs() == 1 {
                        if grid.occupants.contains_key(&(*x, ny))
                            || grid.occupants.contains_key(&(nx, *y))
                        {
                            blocked = true;
                        }
                    }

                    if !blocked {
                        if let Some(&_neighbor_entity) = grid.tiles.get(&(nx, ny)) {
                            graph_neighbors.push((nx, ny));
                        }
                    }
                }
            }
        }

        nav_graph.nodes.insert((*x, *y), graph_neighbors);
    }

    info!("NavGraph generated with {} nodes.", nav_graph.nodes.len());
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

    while let Some(State { cost: _, position }) = heap.pop() {
        visited_count += 1;

        if position == goal {
            // info!("Path found! Visited {} nodes.", visited_count);
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

        if let Some(neighbors) = graph.nodes.get(&position) {
            for &neighbor in neighbors {
                let current_g = *dist.get(&position).unwrap();

                // Calculate cost based on distance (10 for cardinal, 14 for diagonal)
                let dx = (neighbor.0 as i32 - position.0 as i32).abs();
                let dy = (neighbor.1 as i32 - position.1 as i32).abs();
                let step_cost = if dx + dy == 2 { 14 } else { 10 };

                let new_cost = current_g + step_cost;

                let neighbor_dist = *dist.get(&neighbor).unwrap_or(&u32::MAX);

                if new_cost < neighbor_dist {
                    dist.insert(neighbor, new_cost);
                    // Heuristic: Euclidean distance * 10 (to match scale)
                    let h_dx = (neighbor.0 as i32 - goal.0 as i32).abs() as f32;
                    let h_dy = (neighbor.1 as i32 - goal.1 as i32).abs() as f32;
                    let h = ((h_dx * h_dx + h_dy * h_dy).sqrt() * 10.0) as u32;

                    heap.push(State {
                        cost: new_cost + h,
                        position: neighbor,
                    });
                    came_from.insert(neighbor, position);
                }
            }
        } else {
            // warn!("Node {:?} has no entry in graph!", position);
        }
    }

    /*
    info!(
        "Pathfinding failed. Visited {} nodes. Graph size: {}",
        visited_count,
        graph.nodes.len()
    );
    */
    None
}

fn get_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    let mut dx = (x1 - x0).abs();
    let mut dy = -(y1 - y0).abs();
    let mut x = x0;
    let mut y = y0;
    let mut sx = if x0 < x1 { 1 } else { -1 };
    let mut sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        points.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    points
}

pub fn has_line_of_sight(
    start: Vec3,
    end: Vec3,
    config: &crate::arena::ArenaConfig,
    grid: &crate::arena::ArenaGrid,
) -> bool {
    let x1 = ((start.x - config.tile_size * 0.5) / config.tile_size).floor() as i32;
    let z1 = ((start.z - config.tile_size * 0.5) / config.tile_size).floor() as i32;
    let x2 = ((end.x - config.tile_size * 0.5) / config.tile_size).floor() as i32;
    let z2 = ((end.z - config.tile_size * 0.5) / config.tile_size).floor() as i32;

    let points = get_line(x1, z1, x2, z2);

    for (x, z) in points {
        // Skip start tile
        if x == x1 && z == z1 {
            continue;
        }

        // Check bounds
        if x < 0 || x >= config.width as i32 || z < 0 || z >= config.height as i32 {
            return false; // Out of bounds is blocking
        }

        // Check occupants
        if grid.occupants.contains_key(&(x as u32, z as u32)) {
            return false; // Blocked
        }
    }

    true
}
