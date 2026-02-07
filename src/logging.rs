use crate::arena::areas::AreaID;
use crate::arena::CollectibleType;
use crate::building::StructureType;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    AreaEntered {
        entity: Entity,
        area_id: AreaID,
        time: f32,
    },
    StructureBuilt {
        entity: Entity,
        structure: StructureType,
        location: (u32, u32),
        time: f32,
    },
    StructureDestroyed {
        destroyer: Option<Entity>,
        structure: StructureType,
        location: (u32, u32),
        time: f32,
    },
    ItemCollected {
        entity: Entity,
        item_type: CollectibleType,
        location: (u32, u32),
        time: f32,
    },
    DamageDealt {
        attacker: Entity,
        victim: Entity,
        amount: u32,
        time: f32,
    },
    PlayerEliminated {
        entity: Entity,
        killer: Option<Entity>,
        time: f32,
    },
}

#[derive(Resource, Default)]
pub struct MatchLog {
    pub events: Vec<GameEvent>,
}

impl MatchLog {
    pub fn add(&mut self, event: GameEvent) {
        // info!("Game Event: {:?}", event);
        self.events.push(event);
    }
}

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MatchLog>();
    }
}
