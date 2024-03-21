
mod dev_api;
mod test_commands;

use std::{any::TypeId, str::FromStr};
use std::fmt::Debug;
use bevy::app::AppExit;
use bevy::ecs::world::Command;
use bevy::reflect::TypeInfo;
use bevy::utils::{HashMap, HashSet};
use bevy::{prelude::*, reflect::GetTypeRegistration};


use dev_api::*;
use rustyline::error::ReadlineError;
use test_commands::{Gold, SetGold};

#[derive(Resource, Deref, DerefMut)]
struct Console(rustyline::DefaultEditor);

fn main() {
    let rl = rustyline::DefaultEditor::new().unwrap();
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Console(rl))
        .add_systems(Update, read_console)
        
        //setup toolbox
        .insert_resource(CLIToolBox::default())
        .add_systems(Startup, setup)

        .insert_resource(Gold::default())
        .run();
}

fn setup(
    mut commands: Commands,
    mut toolbox: ResMut<CLIToolBox>,
) {
    toolbox.add_tool::<SetGold>();

    toolbox.direct_applyer::<u64, _>();
}

fn read_console(
    mut commands: Commands,
    mut console: ResMut<Console>,
    toolbox: ResMut<CLIToolBox>,
    mut ev_app_event: EventWriter<AppExit>,) 
{
    let result_input = console.readline("> ");

    let input = match result_input {
        Ok(input) => input,
        Err(ReadlineError::Interrupted) => {
            println!("CTRL-C");
            ev_app_event.send(AppExit);
            return;
        },
        Err(ReadlineError::Eof) => {
            println!("CTRL-D");
            ev_app_event.send(AppExit);
            return;
        },
        Err(err) => {
            println!("Error: {:?}", err);
            return;
        }
    };

    if let Ok((command, metadata)) = toolbox.parse_dev_command(&input) {
        info!("Command: parsed {:?}", command);
        (metadata.add_self_to_commands_fn)(&mut commands, command.as_ref());
        
    } else {
        error!("Invalid command: {:?}", input);
    }
}

//Examlpe CLI parser
#[derive(Default, Resource)]
/// Resource that contains metadata about all of the CLI tools.
pub struct CLIToolBox {
    /// Metadata about all of the available dev commands.
    pub metadatas: HashMap<String, DevCommandMetadata>,

    pub apply_from_string: Vec<fn(&mut dyn Reflect, &str) -> bool>,
    pub metadate_create_fn: HashMap<String, fn() -> DevCommandMetadata>
}

/// Parse a command line input into a DevCommand
impl CLIToolBox {

    pub fn add_tool<T : DevCommand>(&mut self) {
        let metadata = T::metadata();
        info!("Added tool: {}", metadata.name);

        self.metadate_create_fn.insert(metadata.name.to_string(), || T::metadata());
        self.metadatas.insert(metadata.name.to_string(), metadata);
    }

    pub fn direct_applyer<T: FromStr<Err=E> + Reflect, E>(&mut self) {
        self.apply_from_string.push(|command: &mut dyn Reflect, value: &str| {
            let Some(command) = command.downcast_mut::<T>() else {
                return false;
            };

            if let Ok(value) = value.parse::<T>() {
                *command = value;
                true
            } else {
                false
            }
        })
    }

    /// Parse a command line input into a DevCommand
    ///
    /// Takes a string of space separated words and returns a DevCommand if
    /// the input is valid. Otherwise returns a DevToolParseError
    pub fn parse_dev_command(&self, s: &str) -> Result<(Box<dyn Reflect>, DevCommandMetadata), DevToolParseError> {

        let words = s.split_whitespace().collect::<Vec<&str>>();

        // No words in input
        if words.len() == 0 {
            error!("No words in input");
            return Err(DevToolParseError::InvalidName);
        }



        let name = words[0];

        // Look up the metadata for the command
        let Some(metadata) = self.metadatas.get(name) else {
            error!("Unknown command: {}", name);
            return Err(DevToolParseError::InvalidName);
        };

        let mut command = (metadata.create_default_fn)();

        let set_field_by_idx = |command: &mut dyn Reflect, idx: usize, value: &str| {
            let field = match command.reflect_mut() {
                bevy::reflect::ReflectMut::Struct(r) => {
                    let Some(field) = r.field_at_mut(idx) else {
                        error!("Invalid index: {}", idx);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field
                },
                bevy::reflect::ReflectMut::TupleStruct(r) => {
                    let Some(field) = r.field_mut(idx) else {
                        error!("Invalid index: {}", idx);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field
                },
                bevy::reflect::ReflectMut::Tuple(r) => {
                    let Some(field) = r.field_mut(idx) else {
                        error!("Invalid index: {}", idx);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field
                },
                bevy::reflect::ReflectMut::List(r) => {
                    let Some(field) = r.get_mut(idx) else {
                        error!("Invalid index: {}", idx);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field
                },
                bevy::reflect::ReflectMut::Array(r) => {
                    let Some(field) = r.get_mut(idx) else {
                        error!("Invalid index: {}", idx);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field
                },
                bevy::reflect::ReflectMut::Map(r) => {
                    let Some(field) = r.get_at_mut(idx) else {
                        error!("Invalid index: {}", idx);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field.1
                },
                bevy::reflect::ReflectMut::Enum(r) => {
                    let Some(field) = r.field_at_mut(idx) else {
                        error!("Invalid index: {}", idx);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field
                },
                bevy::reflect::ReflectMut::Value(r) => r,
            };

            for applyer in self.apply_from_string.iter() {
                if applyer(field, value) {
                    return Ok(());
                }
            }

            error!("No applyer found for field: {}", name);
            Err(DevToolParseError::InvalidToolData)
        };

        let set_field_by_name = |command: &mut dyn Reflect, name: &str, value: &str| {
            let field = match command.reflect_mut() {
                bevy::reflect::ReflectMut::Struct(r) => {
                    let Some(field) = r.field_mut(name) else {
                        error!("Invalid name: {}", name);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field
                },
                bevy::reflect::ReflectMut::TupleStruct(r) => {
                    error!("Not support named fields in tuple structs: {}", name);
                    return Err(DevToolParseError::InvalidToolData);
                },
                bevy::reflect::ReflectMut::Tuple(r) => {
                    error!("Not support named fields in tuples: {}", name); 
                    return Err(DevToolParseError::InvalidToolData);
                },
                bevy::reflect::ReflectMut::List(r) => {
                    error!("Not support named fields in lists: {}", name);
                    return Err(DevToolParseError::InvalidToolData);
                },
                bevy::reflect::ReflectMut::Array(r) => {
                    error!("Not support named fields in arrays: {}", name);
                    return Err(DevToolParseError::InvalidToolData);
                },
                bevy::reflect::ReflectMut::Map(r) => {
                    error!("Not support named fields in maps: {}", name);
                    return Err(DevToolParseError::InvalidToolData);
                },
                bevy::reflect::ReflectMut::Enum(r) => {
                    let Some(field) = r.field_mut(name) else {
                        error!("Invalid name: {}", name);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    field
                },
                bevy::reflect::ReflectMut::Value(r) => {
                    error!("Not support named fields in values: {}", name);
                    return Err(DevToolParseError::InvalidToolData);
                },
            };

            for applyer in self.apply_from_string.iter() {
                if applyer(field, value) {
                    return Ok(());
                }
            }

            error!("No applyer found for field: {}", name);
            Err(DevToolParseError::InvalidToolData)
        };
        
        // The current named parameter being parsed
        let mut named_param = None;
        // Whether or not we are currently in named style
        let mut is_named_style = false;
        // Index of the next parameter to expect in positional style
        let mut idx = 0;

        for word in words.iter().skip(1) {
            if word.starts_with("--") {
                is_named_style = true;
                named_param = Some(word.to_string());
            } else {
                if is_named_style {
                    let Some(named_param) = &named_param else {
                        error!("Not fount name for value: {}", word);
                        return Err(DevToolParseError::InvalidToolData);
                    };
                    set_field_by_name(command.as_mut(), named_param, word)?;
                } else {
                    set_field_by_idx(command.as_mut(), idx, word)?;
                    idx += 1;
                }
            }
        }

        Ok((command, (self.metadate_create_fn[&metadata.name.to_string()])()))
    }
}