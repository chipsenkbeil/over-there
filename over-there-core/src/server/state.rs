use crate::{msg::callback::CallbackManager, state::State};

#[derive(Debug)]
pub struct ServerState {
    callback_manager: CallbackManager,
}

impl State for ServerState {
    fn callback_manager(&mut self) -> &mut CallbackManager {
        &mut self.callback_manager
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            callback_manager: CallbackManager::default(),
        }
    }
}
