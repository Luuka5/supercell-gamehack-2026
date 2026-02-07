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
    let x1 = (start.x / config.tile_size).round() as i32;
    let z1 = (start.z / config.tile_size).round() as i32;
    let x2 = (end.x / config.tile_size).round() as i32;
    let z2 = (end.z / config.tile_size).round() as i32;

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
