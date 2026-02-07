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
    pub center: (u32, u32),
    pub neighbors: Vec<AreaID>,
    pub visible_areas: Vec<AreaID>,
}

impl Area {
    pub fn new(id: AreaID, min_x: u32, min_y: u32, max_x: u32, max_y: u32) -> Self {
        Self {
            id,
            min_x,
            min_y,
            max_x,
            max_y,
            center: ((min_x + max_x) / 2, (min_y + max_y) / 2),
            neighbors: Vec::new(),
            visible_areas: Vec::new(),
        }
    }

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
        Self { areas: Vec::new() }
    }
}

impl AreaMap {
    pub fn new(areas: Vec<Area>) -> Self {
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
                return Some(area.center);
            }
        }
        None
    }
}
