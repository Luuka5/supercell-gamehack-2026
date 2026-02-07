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
    AiDecision {
        entity: Entity,
        entity_name: String,
        rule_name: String,
        condition_met: bool,
        inventory_obstacles: u32,
        inventory_turrets: u32,
        visible_enemies: usize,
        time: f32,
    },
}

#[derive(Resource, Default)]
pub struct MatchLog {
    pub events: Vec<GameEvent>,
}

impl MatchLog {
    pub fn add(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    pub fn write_to_file(&self, filename: &str) {
        let json = serde_json::to_string_pretty(&self.events).unwrap();
        std::fs::write(filename, json).unwrap();
    }
}

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MatchLog>()
            .add_systems(Update, write_logs_periodically);
    }
}

fn write_logs_periodically(mut timer: Local<f32>, time: Res<Time>, match_log: Res<MatchLog>) {
    *timer += time.delta_secs();
    if *timer > 10.0 {
        *timer = 0.0;
        match_log.write_to_file("game_logs.json");
    }
}
