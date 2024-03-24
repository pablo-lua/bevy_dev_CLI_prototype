use bevy::ecs::world::{Command, CommandQueue};
use bevy::reflect::{FromType, TypeData, TypeInfo, Typed};
use bevy::{prelude::*, reflect::GetTypeRegistration};
use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::{any::TypeId, str::FromStr};

#[derive(Debug)]
pub enum DevToolParseError {
    InvalidName,
    InvalidToolData,
}

/// Modal dev tools are used by developers to inspect their application in a toggleable way,
/// such as an FPS meter or a fly camera.
///
/// Their configuration is stored as a resource (the type that this trait is implemented for),
/// and they can be enabled, disabled and reconfigured at runtime.
///
/// The documentation on this struct is reflected, and can be read by toolboxes to provide help text to users.
pub trait ModalDevTool:
    Resource
    + Reflect
    + FromReflect
    + GetTypeRegistration
    + Default
    + FromStr<Err = DevToolParseError>
    + Debug
{
    /// The name of this tool, as might be supplied by a command line interface.
    fn name() -> &'static str {
        Self::get_type_registration()
            .type_info()
            .type_path_table()
            .short_path()
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
            from_str_fn: |s| {
                <Self as FromStr>::from_str(s).map(|x| Box::new(x) as Box<dyn Reflect>)
            },
            short_description: Self::short_description(),
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
    fn toggle(&mut self) {
        if self.is_enabled() {
            self.disable();
        } else {
            self.enable();
        }
    }
}

pub struct DevToolMetaData {
    pub name: &'static str,
    pub type_id: TypeId,
    pub type_info: &'static TypeInfo,
    pub from_str_fn: fn(&str) -> Result<Box<dyn Reflect>, DevToolParseError>,
    pub short_description: Option<&'static str>,
}

/// Dev commands are used by developers to modify the `World` in order to easily debug and test their application.
///
/// Dev commands can be called with arguments to specify the exact behavior: if you are creating a toolbox, parse the provided arguments
/// to construct an instance of the type that implements this type, and then send it as a `Command` to execute it.
///
/// The documentation on this struct is reflected, and can be read by toolboxes to provide help text to users.
pub trait DevCommand:
    bevy::ecs::world::Command
    + Reflect
    + FromReflect
    + GetTypeRegistration
    + Default
    + FromStr<Err = DevToolParseError>
    + Debug
    + 'static
{
    /// The name of this tool, as might be supplied by a command line interface.
    fn name() -> &'static str {
        Self::get_type_registration()
            .type_info()
            .type_path_table()
            .short_path()
    }

    fn short_description() -> Option<&'static str>;

    /// The metadata for this dev command.
    fn metadata() -> DevCommandMetadata {
        DevCommandMetadata {
            name: Self::name(),
            type_id: Self::get_type_registration().type_id(),
            type_info: Self::get_type_registration().type_info(),
            // A function pointer, based on the std::str::from_str method
            from_str_fn: |s| {
                <Self as FromStr>::from_str(s).map(|x| Box::new(x) as Box<dyn Reflect>)
            },
            create_default_fn: || Box::new(Self::default()),
            add_self_to_commands_fn: |commands, reflected_self| {
                commands.add(<Self as FromReflect>::from_reflect(reflected_self).unwrap())
            },
            short_description: Self::short_description(),
        }
    }
}

pub struct DevCommandMetadata {
    pub name: &'static str,
    pub type_id: TypeId,
    pub type_info: &'static TypeInfo,
    pub from_str_fn: fn(&str) -> Result<Box<dyn Reflect>, DevToolParseError>,
    pub create_default_fn: fn() -> Box<dyn Reflect>,
    pub add_self_to_commands_fn: fn(commands: &mut Commands, reflected_self: &dyn Reflect),
    pub short_description: Option<&'static str>,
}

pub trait AppRegisterToolExt {
    fn register_dev_tool<T: GetTypeRegistration>(&mut self);

    fn register_dev_command<T: GetTypeRegistration>(&mut self);
}
