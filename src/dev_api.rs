use std::any::Any;
use std::{any::TypeId, str::FromStr};
use std::fmt::Debug;
use bevy::reflect::TypeInfo;
use bevy::{prelude::*, reflect::GetTypeRegistration};



pub enum DevToolParseError {
    InvalidName,
    InvalidArgument(String),
}

/// Modal dev tools are used by developers to inspect their application in a toggleable way,
/// such as an FPS meter or a fly camera.
/// 
/// Their configuration is stored as a resource (the type that this trait is implemented for),
/// and they can be enabled, disabled and reconfigured at runtime.
/// 
/// The documentation on this struct is reflected, and can be read by toolboxes to provide help text to users.
pub trait ModalDevTool: Resource + Reflect + FromReflect + GetTypeRegistration + FromStr<Err=DevToolParseError> + Debug {
    /// The name of this tool, as might be supplied by a command line interface.
    fn name() -> &'static str {
        Self::get_type_registration().type_info().type_path_table().short_path()
    }
    
    fn short_description() -> Option<&'static str> {
        None
    }

    /// The metadata for this modal dev tool.
    fn metadata() -> DevToolMetaData {
        DevToolMetaData {
            name: Self::name(),
            type_id: Self::get_type_registration().type_id(),
            type_info: Self::get_type_registration().type_info(),
            // A function pointer, based on the std::str::from_str method
            from_str_fn: |s| <Self as FromStr>::from_str(s).map(|x| Box::new(x) as Box<dyn Reflect>),
            short_description: Self::short_description()
        }
    }

    /// Turns this dev tool on (true) or off (false).
    fn set_enabled(&mut self, enabled: bool);

    /// Is this dev tool currently enabled?
    fn is_enabled(&self) -> bool;

    /// Enables this dev tool.
    fn enable(&mut self) {
        self.set_enabled(true);
    }

    /// Disables this dev tool.
    fn disable(&mut self) {
        self.set_enabled(false);
    }

    /// Enables this dev tool if it's disabled, or disables it if it's enabled.
    fn toggle(&mut self){
        if self.is_enabled(){
            self.disable();
        } else {
            self.enable();
        }
    }
}

pub struct DevToolMetaData {
    name: &'static str,
    type_id: TypeId,
    type_info: &'static TypeInfo,
    from_str_fn: fn(&str) -> Result<Box<dyn Reflect>, DevToolParseError>,
    short_description: Option<&'static str>
}

/// Dev commands are used by developers to modify the `World` in order to easily debug and test their application.
/// 
/// Dev commands can be called with arguments to specify the exact behavior: if you are creating a toolbox, parse the provided arguments 
/// to construct an instance of the type that implements this type, and then send it as a `Command` to execute it.
/// 
/// The documentation on this struct is reflected, and can be read by toolboxes to provide help text to users.
pub trait DevCommand: bevy::ecs::world::Command + Reflect + FromReflect + GetTypeRegistration + FromStr<Err=DevToolParseError> + Debug + 'static {
    /// The name of this tool, as might be supplied by a command line interface.
    fn name() -> &'static str {
        Self::get_type_registration().type_info().type_path_table().short_path()
    }

    fn short_description() -> Option<&'static str>;

    /// The metadata for this dev command.
    fn metadata() -> DevCommandMetadata {
        DevCommandMetadata {
            name: Self::name(),
            type_id: Self::get_type_registration().type_id(),
            type_info: Self::get_type_registration().type_info(),
            // A function pointer, based on the std::str::from_str method
            from_str_fn: |s| <Self as FromStr>::from_str(s).map(|x| Box::new(x) as Box<dyn Reflect>),
            short_description: Self::short_description()
        }
    }
}

pub struct DevCommandMetadata {
    name: &'static str,
    type_id: TypeId,
    type_info: &'static TypeInfo,
    from_str_fn: fn(&str) -> Result<Box<dyn Reflect>, DevToolParseError>,
    short_description: Option<&'static str>
}