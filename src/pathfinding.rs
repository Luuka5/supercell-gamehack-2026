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
