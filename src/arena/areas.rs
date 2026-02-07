use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum AreaID {
    UserBase,
    EnemyBase,
    CenterArena,
    NorthCorridor,
    SouthCorridor,
    Unknown,
}

#[derive(Debug, Clone, Reflect)]
pub struct Area {
    pub id: AreaID,
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl Area {
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

#[derive(Resource)]
pub struct AreaMap {
    pub areas: Vec<Area>,
}

impl Default for AreaMap {
    fn default() -> Self {
        Self::new()
    }
}

impl AreaMap {
    pub fn new() -> Self {
        let mut areas = Vec::new();

        // User Base (Top Leftish - based on start pos (4, 2))
        // Let's say 0-10, 0-10
        areas.push(Area {
            id: AreaID::UserBase,
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
        });

        // Enemy Base (Bottom Rightish - based on start pos (35, 2))
        // Wait, (35, 2) is also low Y. So both bases are at the "bottom" (low Z)?
        // Let's check ARENA_LAYOUT again.
        // It's 40 wide (X), 28 high (Z).
        // User (4, 2). Enemy (35, 2).
        // So they are on the same "row" (Z=2), but opposite sides of X.
        // So it's Left vs Right, not Top vs Bottom.

        areas.push(Area {
            id: AreaID::EnemyBase,
            min_x: 30,
            min_y: 0,
            max_x: 39,
            max_y: 10,
        });

        // Center Arena
        areas.push(Area {
            id: AreaID::CenterArena,
            min_x: 11,
            min_y: 0,
            max_x: 29,
            max_y: 27,
        });

        // North Corridor (High Z)
        areas.push(Area {
            id: AreaID::NorthCorridor,
            min_x: 0,
            min_y: 11,
            max_x: 39,
            max_y: 27,
        });

        Self { areas }
    }

    pub fn get_area_id(&self, x: u32, y: u32) -> AreaID {
        for area in &self.areas {
            if area.contains(x, y) {
                return area.id.clone();
            }
        }
        AreaID::Unknown
    }

    pub fn get_center(&self, id: AreaID) -> Option<(u32, u32)> {
        for area in &self.areas {
            if area.id == id {
                return Some(((area.min_x + area.max_x) / 2, (area.min_y + area.max_y) / 2));
            }
        }
        None
    }
}
