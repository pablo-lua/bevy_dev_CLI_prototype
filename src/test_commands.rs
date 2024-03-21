use std::str::FromStr;

use crate::dev_api::*;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Gold(u64);

/// Sets the player's gold to the provided value.
#[derive(Reflect, Debug, Default)]
pub struct SetGold {
    pub amount: u64,
}

impl bevy::ecs::world::Command for SetGold {
    fn apply(self, world: &mut World) {
        let mut current_gold = world.resource_mut::<Gold>();
        current_gold.0 = self.amount;

        info!("Set gold to {}", current_gold.0);
    }
}

impl DevCommand for SetGold {
    fn short_description() -> Option<&'static str> {
        Some("Sets the player's gold to the provided value.")
    }
}

impl FromStr for SetGold {
    type Err = DevToolParseError;
    fn from_str(s: &str) -> Result<Self, DevToolParseError>{
        let mut parts = s.split_whitespace();
        //return error if name if none
        let Some(name) = parts.next() else {
            return Err(DevToolParseError::InvalidName);
        };
        if name != Self::name() {
            return Err(DevToolParseError::InvalidName);
        }

        let Some(amount_string) = parts.next() else {
            return Err(DevToolParseError::InvalidToolData);
        };
        let amount = amount_string.parse().map_err(|_| DevToolParseError::InvalidToolData)?;

        Ok(SetGold {amount} )
    }
}