
mod dev_api;
mod test_commands;

use std::{any::TypeId, str::FromStr};
use std::fmt::Debug;
use bevy::reflect::TypeInfo;
use bevy::{prelude::*, reflect::GetTypeRegistration};


use dev_api::*;

fn main() {
    println!("Hello, world!");
}

//Examlpe CLI parser
#[derive(Default, Resource)]
/// Resource that contains metadata about all of the CLI tools.
pub struct CLIToolBox {
    /// Metadata about all of the available dev commands.
    pub metadatas: Vec<DevCommandMetadata>,
}

// /// Parse a command line input into a DevCommand
// impl CLIDevBox {
//     /// Parse a command line input into a DevCommand
//     ///
//     /// Takes a string of space separated words and returns a DevCommand if
//     /// the input is valid. Otherwise returns a DevToolParseError
//     pub fn parse_dev_command(&self, s: &str) -> Result<Box<dyn Reflect>, DevToolParseError> {

//         let words = s.split_whitespace().collect::<Vec<&str>>();

//         // No words in input
//         if words.len() == 0 {
//             return Err(DevToolParseError("No words".to_string()));
//         }



//         let name = words[0];

//         // Look up the metadata for the command
//         let Some(cli_metadata) = self.cli_metadatas.get(name) else {
//             return Err(DevToolParseError(format!("Unknown command: {}", name)));
//         };

//         let param_desc = &cli_metadata.params_description;
//         let mut params = HashMap::new();
        
//         // The current named parameter being parsed
//         let mut named_param = None;
//         // Hashset of all named parameters parsed
//         let mut named_set = HashSet::new();
//         // Whether or not we are currently in named style
//         let mut is_named_style = false;
//         // Index of the next parameter to expect in positional style
//         let mut idx = 0;

//         for word in words.iter().skip(1) {
//             if word.starts_with("--") {
//                 is_named_style = true;
//                 named_param = Some(word.to_string());
//                 named_set.insert(word.to_string());
//             } else {
//                 if is_named_style {
//                     let Some(named_param) = &named_param else {
//                         return Err(DevToolParseError("Expected named parameter".to_string()));
//                     };
//                     params.insert(named_param.clone(), word.to_string());
//                 } else {
//                     let desc = &param_desc[idx];
//                     params.insert(desc.name.to_string(), word.to_string());
//                     idx += 1;
//                     if idx >= param_desc.len() {
//                         break;
//                     }
//                 }
//             }
//         }

//         //check params condition
//         for param in param_desc.iter() {
//             if let Some(value) = params.get(&param.name.to_string()) {
//                 //check type
//                 if let Some(checker) = &param.type_checker {
//                     if !(checker.check_fn)(value) {
//                         return Err(DevToolParseError(format!("Expected {} to be {}", param.name, checker.type_name)));
//                     }
//                 }

//                 //check position
//                 if param.position == ParamPosition::Named && !named_set.contains(&param.name.to_string()) {
//                     return Err(DevToolParseError(format!("Missing named parameter {}", param.name)));
//                 } else if param.position == ParamPosition::Positional && named_set.contains(&param.name.to_string()) {
//                     return Err(DevToolParseError(format!("Unexpected named parameter {}", param.name)));
//                 }
//             } else if param.required {
//                 return Err(DevToolParseError(format!("Missing required parameter {}", param.name)));
//             }
//         }

//         (cli_metadata.from_params)(params)
//     }
// }