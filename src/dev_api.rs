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

pub trait ModalDevTl: Resource + Reflect + GetTypeRegistration + FromReflect {
    fn name() -> &'static str {
        Self::get_type_registration()
            .type_info()
            .type_path_table()
            .short_path()
    }

    fn metadata() {

    }
}

pub trait Toggleable: ModalDevTl {
    fn is_enabled(&self) -> bool;

    fn set_enabled(&mut self, enable: bool);

    fn disable(&mut self) {
        self.set_enabled(false);
    }

    fn enable(&mut self) {
        self.set_enabled(true);
    }

    fn toggle(&mut self) {
        if self.is_enabled() {
            self.disable()
        } else {
            self.enable()
        }
    }
}

#[derive(Debug, Reflect)]
pub struct Disable<T: Toggleable>(PhantomData<T>);
impl <T: Toggleable>Command for Disable<T> {
    fn apply(self, world: &mut World) {
        if let Some(mut tool) = world.get_resource_mut::<T>() {
            tool.disable()
        } else {
            warn!("Couldn't find {} in resources", T::name())
        }
    }
}

pub struct Enable<T: Toggleable>(PhantomData<T>);
impl <T: Toggleable> Command for Enable<T> {
    fn apply(self, world: &mut World) {
        if let Some(mut tool) = world.get_resource_mut::<T>() {
            tool.disable()
        } else {
            warn!("Couldn't find {} in resources", T::name())
        }
    }
}

pub struct Toggle<T: Toggleable>(PhantomData<T>);
impl <T: Toggleable> Command for Toggle<T> {
    fn apply(self, world: &mut World) {
        if let Some(mut tool) = world.get_resource_mut::<T>() {
            tool.toggle()
        } else {
            warn!("Couldn't find {} in resources", T::name())
        }
    }
}

#[derive(Clone)]
pub struct ReflectToggle {
    pub insert_enable: fn(&mut Commands),
    pub insert_toggle: fn(&mut Commands),
    pub insert_disable: fn(&mut Commands)
}

impl ReflectToggle {
    pub fn toggle_from_world(&self, world: &mut World) {
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, world);
        (self.insert_toggle)(&mut commands);
        command_queue.apply(world);
    }

    pub fn enable_from_world(&self, world: &mut World) {
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, world);
        (self.insert_enable)(&mut commands);
        command_queue.apply(world);
    }

    pub fn disable_from_world(&self, world: &mut World) {
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, world);
        (self.insert_disable)(&mut commands);
        command_queue.apply(world);
    }

}

impl <T: Reflect + Toggleable + FromReflect>FromType<T> for ReflectToggle {
    fn from_type() -> Self {
        ReflectToggle {
            insert_disable: |commands: &mut Commands| {
                let disable: Disable<T> = Disable(PhantomData);
                commands.add(disable);
            },

            insert_enable: |commands: &mut Commands| {
                let enable: Enable<T> = Enable(PhantomData);
                commands.add(enable);
            },

            insert_toggle: |commands: &mut Commands| {
                let toggle: Toggle<T> = Toggle(PhantomData);
                commands.add(toggle);
            }
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
