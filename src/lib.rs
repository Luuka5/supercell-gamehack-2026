//! # Bevy Test Shared Library
//!
//! This crate defines only the top-level WebSocket message enums.
//! All other shared types are defined in their respective modules within the client codebase.

use serde::{Deserialize, Serialize};

// Re-export the necessary types for the message enums.
use crate::ai::rules::RuleSet;
use crate::logging::MatchLog;
use crate::player::PlayerStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    UpdatePlayerStatus(PlayerStatus),
    PushMatchLog(MatchLog),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    UpdateRuleSet(RuleSet),
}
