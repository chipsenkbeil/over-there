use crate::{msg::callback::CallbackManager, state::State};
use std::fmt::Debug;
use std::time::Instant;

#[derive(Debug)]
pub struct ClientState {
    /// Contains the time when the last heartbeat was received
    /// from the remote instance
    pub last_heartbeat: Instant,

    /// Contains the version of the remote instance
    pub remote_version: String,

    callback_manager: CallbackManager,
}

impl State for ClientState {
    fn callback_manager(&mut self) -> &mut CallbackManager {
        &mut self.callback_manager
    }
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            last_heartbeat: Instant::now(),
            remote_version: String::default(),
            callback_manager: CallbackManager::default(),
        }
    }
}
