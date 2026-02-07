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

impl RuleSet {
    pub fn new_turret_only() -> Self {
        Self {
            rules: vec![
                // Rule for debugging: Always try to build a turret if available.
                Rule {
                    name: "DebugBuildTurret".to_string(),
                    priority: 100,
                    condition: Condition::HasItem {
                        item: "turret".to_string(),
                        count: 1,
                    },
                    action: Action::Build {
                        structure: StructureType::Turret,
                        direction: None,
                    },
                },
            ],
        }
    }
}

impl Default for RuleSet {
    fn default() -> Self {
        Self {
            rules: vec![
                // Rule 1: Retreat to Safety (Highest Priority)
                Rule {
                    name: "RetreatToSafety".to_string(),
                    priority: 100,
                    condition: Condition::IsHealthLow { threshold: 1 },
                    action: Action::MoveToArea(AreaID("EnemyBase".to_string())),
                },
                // Rule 2: Deploy Combat Turret (High Priority - must be higher than EngageEnemy)
                Rule {
                    name: "DeployCombatTurret".to_string(),
                    priority: 95,
                    condition: Condition::And(vec![
                        Condition::IsEnemyVisible,
                        Condition::HasItem {
                            item: "turret".to_string(),
                            count: 1,
                        },
                    ]),
                    action: Action::Build {
                        structure: StructureType::Turret,
                        direction: None,
                    },
                },
                // Rule 3: Engage Enemy (Medium-High Priority)
                Rule {
                    name: "EngageEnemy".to_string(),
                    priority: 80,
                    condition: Condition::IsEnemyVisible,
                    action: Action::ChaseEnemy,
                },
                // Rule 4: Fortify Center (Medium Priority)
                Rule {
                    name: "FortifyCenter".to_string(),
                    priority: 50,
                    condition: Condition::And(vec![
                        Condition::InArea(AreaID("CenterArena".to_string())),
                        Condition::HasItem {
                            item: "obstacle".to_string(),
                            count: 1,
                        },
                    ]),
                    action: Action::Build {
                        structure: StructureType::Obstacle,
                        direction: None,
                    },
                },
                // Rule 5: Claim Center (Low Priority)
                Rule {
                    name: "ClaimCenter".to_string(),
                    priority: 20,
                    condition: Condition::Not(Box::new(Condition::InArea(AreaID(
                        "CenterArena".to_string(),
                    )))),
                    action: Action::MoveToArea(AreaID("CenterArena".to_string())),
                },
                // Rule 6: Invade Player Base (Lowest Priority)
                Rule {
                    name: "InvadePlayerBase".to_string(),
                    priority: 10,
                    condition: Condition::InArea(AreaID("CenterArena".to_string())),
                    action: Action::MoveToArea(AreaID("UserBase".to_string())),
                },
            ],
        }
    }
}
