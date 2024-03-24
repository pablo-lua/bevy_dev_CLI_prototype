mod dev_api;
mod dev_terminal;
mod test_commands;
mod test_tool;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::{app::AppExit, ecs::world::CommandQueue, reflect::GetTypeRegistration};
use dev_terminal::{CLIToolPlugin, Console, ReadedLine, SafeInput};
use test_tool::DevFlyCamera;
use std::{
    any::{Any, TypeId},
    str::FromStr,
    sync::Arc,
};

use dev_api::*;
use test_commands::{Gold, PrintGold, SetGold};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CLIToolPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            update_terminal.run_if(|input_receiver: Res<SafeInput>| input_receiver.is_none()),
        )
        .add_systems(
            Update, readed_lines
        )
        .init_resource::<CLIToolBox>()
        .init_resource::<Gold>()
        .init_resource::<DevFlyCamera>()
        //setup toolbox
        .run();
}

// Asks for a new line when terminal is available
fn update_terminal(console: ResMut<Console>, mut input_receiver: ResMut<SafeInput>) {
    if input_receiver.is_none() {
        console.read_new_line(&mut *input_receiver)
    }
}

fn readed_lines(world: &mut World) {
    let mut state: SystemState<EventReader<ReadedLine>> = SystemState::new(world);
    let mut to_read_strings = Vec::new();
    // Just avoiding borrow checker
    {
        let mut reader = state.get_mut(world);
        for event in reader.read() {
            let Ok(input) = event.0.clone() else {
                continue;
            };
            to_read_strings.push(input);
        }
    }
    if !to_read_strings.is_empty() {
        world.resource_scope(|world: &mut World, cli_toolbox: Mut<CLIToolBox>| {
            for input in to_read_strings.drain(0..to_read_strings.len()) {
                cli_toolbox.parse_input(&input, world);
            }
        })
    }
}

fn setup(mut toolbox: ResMut<CLIToolBox>) {
    toolbox.add_command::<SetGold>();
    toolbox.add_command::<PrintGold>();

    toolbox.add_tool::<test_tool::DevFlyCamera>();

    toolbox.direct_applyer::<u64, _>();
    toolbox.direct_applyer::<bool, _>();

    toolbox.from_parse_fn::<Option<f32>>(Arc::new(|s| {
        if s == "None" {
            Some(None)
        } else if s.starts_with("Some(") && s.ends_with(')') {
            Some(Some(s[5..s.len() - 1].parse().unwrap()))
        } else {
            None
        }
    }));
}

// fn read_console(world: &mut World) {
//     world.resource_scope(|world, mut console: Mut<Console>| {
//         unreachable!();
//         return;
//         let result_input = console.readline("> ");

//         let input = match result_input {
//             Ok(input) => input,
//             Err(ReadlineError::Interrupted) => {
//                 println!("CTRL-C");
//                 world.send_event(AppExit);
//                 return;
//             }
//             Err(ReadlineError::Eof) => {
//                 println!("CTRL-D");
//                 world.send_event(AppExit);
//                 return;
//             }
//             Err(err) => {
//                 println!("Error: {:?}", err);
//                 return;
//             }
//         };

//         world.resource_scope(|world, mut toolbox: Mut<CLIToolBox>| {
//             toolbox.parse_input(&input, world);
//         });
//     });
// }

//Examlpe CLI parser
#[derive(Default, Resource)]
/// Resource that contains metadata about all of the CLI tools.
pub struct CLIToolBox {
    /// Metadata about all of the available dev commands.
    pub metadatas: HashMap<String, DevCommandMetadata>,
    pub metadate_create_fn: HashMap<String, fn() -> DevCommandMetadata>,

    pub tool_metadatas: HashMap<String, DevToolMetaData>,
    pub tool_metadate_create_fn: HashMap<String, fn() -> DevToolMetaData>,
    pub tool_insert_fn: HashMap<
        String,
        fn(&mut World, HashMap<String, Box<dyn Reflect>>) -> Result<(), DevToolParseError>,
    >,
    pub get_tool_fn: HashMap<String, fn(&World) -> &dyn Reflect>,

    pub apply_from_string: Vec<Box<dyn Fn(&mut dyn Reflect, &str) -> bool + Send + Sync>>,
    pub same_from_string_fn: HashMap<
        &'static str,
        Box<
            dyn Fn(&dyn Reflect, &str) -> Result<Box<dyn Reflect>, DevToolParseError> + Send + Sync,
        >,
    >,
}

/// Parse a command line input into a DevCommand
impl CLIToolBox {
    pub fn parse_input(&self, s: &str, world: &mut World) {
        let words = s.split_whitespace().collect::<Vec<&str>>();
        let name = words[0];
        if self.metadatas.contains_key(name) {
            let mut command_queue = CommandQueue::default();
            let mut commands = Commands::new(&mut command_queue, world);
            if let Ok((command, metadata)) = self.parse_dev_command(s) {
                info!("Command: parsed {:?}", command);
                (metadata.add_self_to_commands_fn)(&mut commands, command.as_ref());

                command_queue.apply(world);
            } else {
                error!("Invalid command: {:?}", s);
            }
        } else if self.tool_metadatas.contains_key(name) {
            if let Err(_) = self.update_tool_command(s, world) {
                error!("Invalid tool update: {:?}", s);
            }
        } else {
            error!("Unknown command/tool: {}", name);
        }
    }

    pub fn add_command<T: DevCommand>(&mut self) {
        let metadata = T::metadata();
        info!("Added command: {}", metadata.name);

        self.metadate_create_fn
            .insert(metadata.name.to_string().to_lowercase(), || T::metadata());
        self.metadatas
            .insert(metadata.name.to_string().to_lowercase(), metadata);
    }

    pub fn add_tool<T: ModalDevTool>(&mut self) {
        let metadata = T::metadata();
        info!("Added tool: {}", metadata.name);

        self.tool_metadate_create_fn
            .insert(metadata.name.to_string().to_lowercase(), || T::metadata());
        self.get_tool_fn
            .insert(metadata.name.to_string().to_lowercase(), |world| {
                world.get_resource::<T>().unwrap()
            });
        self.tool_insert_fn
            .insert(metadata.name.to_string().to_lowercase(), |world, patch| {
                let mut tool = world.get_resource_mut::<T>().unwrap();
                for (k, v) in patch {
                    let field = get_field_by_name(tool.as_mut(), &k).unwrap();
                    field.apply(v.as_ref());
                    info!("Set {} to {:?}", k, v);
                }
                Ok(())
            });
        self.tool_metadatas
            .insert(metadata.name.to_string().to_lowercase(), metadata);
    }

    /// Add a direct applyer function to the toolbox.
    ///
    /// This function takes a command that implements `FromStr` and `Reflect`, and
    /// allows us to parse a string into the target.
    pub fn direct_applyer<T: FromStr<Err = E> + Reflect + GetTypeRegistration, E>(&mut self) {
        self.apply_from_string
            .push(Box::new(|target: &mut dyn Reflect, value: &str| {
                let Some(target) = target.downcast_mut::<T>() else {
                    // Couldn't downcast to the provided type, return false.
                    return false;
                };

                if let Ok(value) = value.parse::<T>() {
                    // Parse was successful, set the command to the parsed value and return true.
                    *target = value;
                    true
                } else {
                    // Parse was not successful, return false.
                    false
                }
            }));

        self.same_from_string_fn.insert(
            T::get_type_registration().type_info().type_path(),
            Box::new(|target: &dyn Reflect, value: &str| {
                let Some(_) = target.downcast_ref::<T>() else {
                    // Couldn't downcast to the provided type, return an error.
                    return Err(DevToolParseError::InvalidToolData);
                };
                if let Ok(value) = value.parse::<T>() {
                    // Parse was successful, set the command to the parsed value and return true.
                    Ok(Box::new(value))
                } else {
                    // Parse was not successful, return an error.
                    Err(DevToolParseError::InvalidToolData)
                }
            }),
        );
    }

    /// Create applyer from parse function
    pub fn from_parse_fn<T: Reflect + GetTypeRegistration>(
        &mut self,
        f: Arc<dyn Fn(&str) -> Option<T> + Send + Sync + 'static>,
    ) {
        let moved_f = f.clone();
        self.apply_from_string
            .push(Box::new(move |target: &mut dyn Reflect, value: &str| {
                let Some(target) = target.downcast_mut::<T>() else {
                    // Couldn't downcast to the provided type, return false.
                    return false;
                };

                if let Some(value) = moved_f(value) {
                    // Parse was successful, set the command to the parsed value and return true.
                    *target = value;
                    true
                } else {
                    // Parse was not successful, return false.
                    false
                }
            }));

        self.same_from_string_fn.insert(
            T::get_type_registration().type_info().type_path(),
            Box::new(move |target: &dyn Reflect, value: &str| {
                let Some(_) = target.downcast_ref::<T>() else {
                    // Couldn't downcast to the provided type, return an error.
                    return Err(DevToolParseError::InvalidToolData);
                };
                if let Some(value) = f(value) {
                    // Parse was successful, set the command to the parsed value and return true.
                    Ok(Box::new(value))
                } else {
                    // Parse was not successful, return an error.
                    Err(DevToolParseError::InvalidToolData)
                }
            }),
        );
    }

    /// Parse a command line input into a DevCommand
    ///
    /// Takes a string of space separated words and returns a DevCommand if
    /// the input is valid. Otherwise returns a DevToolParseError
    pub fn parse_dev_command(
        &self,
        s: &str,
    ) -> Result<(Box<dyn Reflect>, DevCommandMetadata), DevToolParseError> {
        let words = s.split_whitespace().collect::<Vec<&str>>();

        // No words in input
        if words.len() == 0 {
            error!("No words in input");
            return Err(DevToolParseError::InvalidName);
        }

        let name = words[0];

        // Look up the metadata for the command
        let Some(metadata) = self.metadatas.get(&name.to_lowercase()) else {
            error!("Unknown command: {}", name);
            return Err(DevToolParseError::InvalidName);
        };

        let mut command = (metadata.create_default_fn)();

        self.parse_reflect_from_cli(words, &mut command)?;

        // Return the command and its metadata
        Ok((
            command,
            (self.metadate_create_fn[&metadata.name.to_lowercase()])(),
        ))
    }

    pub fn update_tool_command(&self, s: &str, world: &mut World) -> Result<(), DevToolParseError> {
        let words = s.split_whitespace().collect::<Vec<&str>>();

        // No words in input
        if words.len() == 0 {
            error!("No words in input");
            return Err(DevToolParseError::InvalidName);
        }

        let name = words[0];

        // Look up the metadata for the command
        let Some(metadata) = self.tool_metadatas.get(&name.to_lowercase()) else {
            error!("Unknown tool: {}", name);
            return Err(DevToolParseError::InvalidName);
        };

        let mut patch = HashMap::new();

        {
            // create brackets to drop tool reference
            let mut tool = (self.get_tool_fn[&metadata.name.to_lowercase()])(world);

            let mut named_param = None;

            for word in words.iter().skip(1) {
                if word.starts_with("--") {
                    named_param = Some(word.trim_start_matches("--").to_string());
                } else {
                    if let Some(named_param) = &named_param {
                        let Ok(field) = get_field_by_name_readonly(tool, named_param) else {
                            error!("Invalid field: {}", named_param);
                            return Err(DevToolParseError::InvalidToolData);
                        };
                        let field_type = field.get_represented_type_info().unwrap().type_path();
                        info!("Field type path: {:?}", field_type);
                        if let Some(applyer) = self.same_from_string_fn.get(&field_type) {
                            if let Ok(field) = applyer(field, word) {
                                patch.insert(named_param.clone(), field);
                            } else {
                                error!("Failed to parse value: {}", word);
                                return Err(DevToolParseError::InvalidToolData);
                            }
                        } else {
                            error!("Failed to find patch for value: {}", word);
                        }
                    }
                }
            }
        }

        (self.tool_insert_fn[&metadata.name.to_lowercase()])(world, patch)?;

        Ok(())
    }

    fn parse_reflect_from_cli(
        &self,
        words: Vec<&str>,
        target: &mut Box<dyn Reflect>,
    ) -> Result<(), DevToolParseError> {
        let mut named_param = None;
        let mut is_named_style = false;
        let mut idx = 0;
        // The current named parameter being parsed
        // Whether or not we are currently in named style
        // Index of the next parameter to expect in positional style

        // Parse all words following the command name
        for word in words.iter().skip(1) {
            // Named style parameter
            if word.starts_with("--") {
                is_named_style = true;
                named_param = Some(word.trim_start_matches("--").to_string());
            } else {
                // Positional style parameter

                // Get the field to apply the value to
                if is_named_style {
                    // Retrieve the named parameter
                    let Some(named_param) = &named_param else {
                        error!("Not found name for value: {}", word);
                        return Err(DevToolParseError::InvalidToolData);
                    };

                    // Find the field with the matching name
                    let Ok(field) = get_field_by_name(target.as_mut(), named_param) else {
                        error!("Invalid name: {}", named_param);
                        return Err(DevToolParseError::InvalidToolData);
                    };

                    // Apply the value to the field
                    let mut ok = false;
                    for applyer in self.apply_from_string.iter() {
                        if applyer(field, &word) {
                            ok = true;
                            break;
                        }
                    }
                    if !ok {
                        error!("Not found applyer for value: {}", word);
                        return Err(DevToolParseError::InvalidToolData);
                    }
                } else {
                    // Find the next field in positional style
                    let Ok(field) = get_field_by_idx(target.as_mut(), idx) else {
                        error!("Invalid index: {}", idx);
                        return Err(DevToolParseError::InvalidToolData);
                    };

                    // Apply the value to the field
                    let mut ok = false;
                    for applyer in self.apply_from_string.iter() {
                        if applyer(field, &word) {
                            ok = true;
                            break;
                        }
                    }
                    if !ok {
                        error!("Not found applyer for value: {}", word);
                        return Err(DevToolParseError::InvalidToolData);
                    }

                    // Increment the index of the next positional style parameter
                    idx += 1;
                }
            }
        }
        Ok(())
    }
}

fn get_field_by_idx<'a>(
    command: &'a mut dyn Reflect,
    idx: usize,
) -> Result<&'a mut dyn Reflect, DevToolParseError> {
    let field = match command.reflect_mut() {
        bevy::reflect::ReflectMut::Struct(r) => {
            let Some(field) = r.field_at_mut(idx) else {
                error!("Invalid index: {}", idx);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectMut::TupleStruct(r) => {
            let Some(field) = r.field_mut(idx) else {
                error!("Invalid index: {}", idx);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectMut::Tuple(r) => {
            let Some(field) = r.field_mut(idx) else {
                error!("Invalid index: {}", idx);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectMut::List(r) => {
            let Some(field) = r.get_mut(idx) else {
                error!("Invalid index: {}", idx);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectMut::Array(r) => {
            let Some(field) = r.get_mut(idx) else {
                error!("Invalid index: {}", idx);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectMut::Map(r) => {
            let Some(field) = r.get_at_mut(idx) else {
                error!("Invalid index: {}", idx);
                return Err(DevToolParseError::InvalidToolData);
            };
            field.1
        }
        bevy::reflect::ReflectMut::Enum(r) => {
            let Some(field) = r.field_at_mut(idx) else {
                error!("Invalid index: {}", idx);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectMut::Value(r) => r,
    };
    Ok(field)
}

fn get_field_by_name<'a>(
    command: &'a mut dyn Reflect,
    name: &str,
) -> Result<&'a mut dyn Reflect, DevToolParseError> {
    let field = match command.reflect_mut() {
        bevy::reflect::ReflectMut::Struct(r) => {
            let Some(field) = r.field_mut(name) else {
                error!("Invalid name: {}", name);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectMut::TupleStruct(r) => {
            error!("Not support named fields in tuple structs: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectMut::Tuple(r) => {
            error!("Not support named fields in tuples: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectMut::List(r) => {
            error!("Not support named fields in lists: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectMut::Array(r) => {
            error!("Not support named fields in arrays: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectMut::Map(r) => {
            error!("Not support named fields in maps: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectMut::Enum(r) => {
            let Some(field) = r.field_mut(name) else {
                error!("Invalid name: {}", name);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectMut::Value(r) => {
            error!("Not support named fields in values: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
    };
    Ok(field)
}

fn get_field_by_name_readonly<'a>(
    command: &'a dyn Reflect,
    name: &str,
) -> Result<&'a dyn Reflect, DevToolParseError> {
    let field = match command.reflect_ref() {
        bevy::reflect::ReflectRef::Struct(r) => {
            let Some(field) = r.field(name) else {
                error!("Invalid name: {}", name);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectRef::TupleStruct(r) => {
            error!("Not support named fields in tuple structs: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectRef::Tuple(r) => {
            error!("Not support named fields in tuples: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectRef::List(r) => {
            error!("Not support named fields in lists: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectRef::Array(r) => {
            error!("Not support named fields in arrays: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectRef::Map(r) => {
            error!("Not support named fields in maps: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
        bevy::reflect::ReflectRef::Enum(r) => {
            let Some(field) = r.field(name) else {
                error!("Invalid name: {}", name);
                return Err(DevToolParseError::InvalidToolData);
            };
            field
        }
        bevy::reflect::ReflectRef::Value(r) => {
            error!("Not support named fields in values: {}", name);
            return Err(DevToolParseError::InvalidToolData);
        }
    };
    Ok(field)
}
