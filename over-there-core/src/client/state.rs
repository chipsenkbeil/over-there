use crate::msg::callback::CallbackManager;
use std::fmt::Debug;
use std::time::Instant;

#[derive(Debug)]
pub struct ClientState {
    /// Contains the time when the last heartbeat was received
    /// from the remote instance
    pub last_heartbeat: Instant,

    /// Contains the version of the remote instance
    pub remote_version: String,

    pub callback_manager: CallbackManager,
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
