use crate::dev_api::*;
use bevy::prelude::*;
use std::str::FromStr;

/// A flying camera controller that lets you disconnect your camera from the player to freely explore the environment.
///
/// When this mode is disabled
#[derive(Resource, Reflect, Debug)]
pub struct DevFlyCamera {
    pub enabled: bool,
    /// How fast the camera travels forwards, backwards, left, right, up and down, in world units.
    pub movement_speed: Option<f32>,
    /// How fast the camera turns, in radians per second.
    pub turn_speed: Option<f32>,
}

impl Default for DevFlyCamera {
    fn default() -> Self {
        DevFlyCamera {
            enabled: false,
            movement_speed: Some(3.),
            turn_speed: Some(10.),
        }
    }
}

impl ModalDevTool for DevFlyCamera {
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl FromStr for DevFlyCamera {
    type Err = DevToolParseError;
    fn from_str(s: &str) -> Result<Self, DevToolParseError> {
        todo!()
    }
}
