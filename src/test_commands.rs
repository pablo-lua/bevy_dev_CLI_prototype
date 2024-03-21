use std::str::FromStr;

use crate::dev_api::*;
use bevy::prelude::*;

#[derive(Resource)]
pub struct Gold(u64);

/// Sets the player's gold to the provided value.
#[derive(Reflect, Debug)]
struct SetGold {
    amount: u64,
}

impl bevy::ecs::world::Command for SetGold {
    fn apply(self, world: &mut World) {
        let mut current_gold = world.resource_mut::<Gold>();
        current_gold.0 = self.amount;
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
            return Err(DevToolParseError::InvalidArgument("amount".to_string()));
        };
        let amount = amount_string.parse().map_err(|_| DevToolParseError::InvalidArgument("amount".to_string()))?;

        Ok(SetGold {amount} )
    }
}