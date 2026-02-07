use crate::arena::areas::AreaID;
use crate::building::StructureType;
use crate::combat::TurretDirection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Condition {
    // Primitives
    True,
    IsEnemyVisible,
    IsHealthLow { threshold: u32 },
    InArea(AreaID),
    HasItem { item: String, count: u32 }, // "obstacle", "turret"
    IsUnderAttack,

    // Composites
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    MoveToArea(AreaID),
    ChaseEnemy,
    Flee,
    Build {
        structure: StructureType,
        direction: Option<TurretDirection>,
    },
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub priority: i32,
    pub condition: Condition,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

impl Default for RuleSet {
    fn default() -> Self {
        Self {
            rules: vec![
                // Rule 1: Flee if health is low
                Rule {
                    name: "FleeLowHealth".to_string(),
                    priority: 100,
                    condition: Condition::IsHealthLow { threshold: 1 },
                    action: Action::Flee,
                },
                // Rule 2: Chase enemy if visible
                Rule {
                    name: "ChaseEnemy".to_string(),
                    priority: 50,
                    condition: Condition::IsEnemyVisible,
                    action: Action::ChaseEnemy,
                },
                // Rule 3: Go to Center (Default)
                Rule {
                    name: "PatrolCenter".to_string(),
                    priority: 10,
                    condition: Condition::True,
                    action: Action::MoveToArea(AreaID::CenterArena),
                },
            ],
        }
    }
}
