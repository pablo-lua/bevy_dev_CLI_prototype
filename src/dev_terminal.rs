use std::sync::{Arc, Mutex};

use bevy::{
    app::Update,
    ecs::{
        event::{Event, EventWriter}, schedule::IntoSystemConfigs, system::{Commands, Res, ResMut, Resource}
    },
    log::{error, warn},
    prelude::{Deref, DerefMut, Plugin},
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use rustyline::{history::DefaultHistory, DefaultEditor, Editor};

#[derive(Resource, Deref, DerefMut)]
pub struct Console(Arc<Mutex<DefaultEditor>>);
impl Default for Console {
    fn default() -> Self {
        Console(Arc::new(Mutex::new(DefaultEditor::new().unwrap())))
    }
}

impl Console {
    pub fn read_new_line(&self, input_receiver: &mut SafeInput) {
        let thread_pool = AsyncComputeTaskPool::get();
        let console_arc: Arc<Mutex<Editor<(), DefaultHistory>>> = self.0.clone();
        let task = thread_pool.spawn(async move {
            if let Ok(mut console_mutex) = console_arc.lock() {
                console_mutex.readline(">").map_err(|err| err.to_string())
            } else {
                // TODO: Create properly an error type
                Err("Algo deu errado".to_string())
            }
        });
        if input_receiver.0.is_none() {
            // We already have a task waiting processing
            **input_receiver = Some(task);
        } else {
            warn!("Sent request for new line without processing old lines");
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct SafeInput(pub Option<Task<Result<String, String>>>);
impl Default for SafeInput {
    fn default() -> Self {
        SafeInput(None)
    }
}

// TODO: Maybe make the reader configurable by user
pub struct CLIToolPlugin;

impl Plugin for CLIToolPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // Console related
        app.init_resource::<Console>()
            .init_resource::<SafeInput>()
            .add_event::<ReadedLine>()
            .add_systems(
                Update,
                // This system will verify any task available
                receive_task.run_if(|input_receiver: Res<SafeInput>| input_receiver.is_some()),
            );
        // ToolBox related
    }
}

// This function handles an active read_line task.
fn receive_task(mut read_lines: EventWriter<ReadedLine>, mut input_receiver: ResMut<SafeInput>) {
    if let Some(task) = input_receiver.0.as_mut() {
        if let Some(result) = block_on(future::poll_once(task)) {
            input_receiver.take();
            read_lines.send(ReadedLine(result));
        }
    }
}

#[derive(Event)]
pub struct ReadedLine(pub Result<String, String>);
