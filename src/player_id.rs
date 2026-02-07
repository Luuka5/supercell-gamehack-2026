//! Defines the PlayerID type for uniquely identifying networked players (human or AI).

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerID(pub u64);

impl PlayerID {
    /// Creates a new, random PlayerID.
    pub fn random() -> Self {
        PlayerID(rand::random())
    }
}
