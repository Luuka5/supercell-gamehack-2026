use crate::arena::areas::AreaID;
use crate::arena::CollectibleType;
use crate::building::StructureType;
use crate::player_id::PlayerID;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    AreaEntered {
        entity: PlayerID,
        area_id: AreaID,
        time: f32,
    },
    StructureBuilt {
        entity: PlayerID,
        structure: StructureType,
        location: (u32, u32),
        time: f32,
    },
    StructureDestroyed {
        destroyer: Option<PlayerID>,
        structure: StructureType,
        location: (u32, u32),
        time: f32,
    },
    ItemCollected {
        entity: PlayerID,
        item_type: CollectibleType,
        location: (u32, u32),
        time: f32,
    },
    DamageDealt {
        attacker: PlayerID,
        victim: PlayerID,
        amount: u32,
        time: f32,
    },
    PlayerEliminated {
        entity: PlayerID,
        killer: Option<PlayerID>,
        time: f32,
    },
    AiDecision {
        entity: PlayerID,
        entity_name: String,
        rule_name: String,
        condition_met: bool,
        inventory_obstacles: u32,
        inventory_turrets: u32,
        visible_enemies: usize,
        time: f32,
    },
}

#[derive(Resource, Default, Debug, Clone, Serialize, Deserialize)]
pub struct MatchLog {
    pub events: Vec<GameEvent>,
}

impl MatchLog {
    pub fn add(&mut self, event: GameEvent) {
        self.events.push(event);
    }
}

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MatchLog>();
    }
}
