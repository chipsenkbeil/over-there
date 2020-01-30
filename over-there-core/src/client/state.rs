use super::file::RemoteFile;
use crate::msg::Msg;
use over_there_utils::CallbackManager;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug)]
pub struct ClientState {
    /// Contains the time when the last heartbeat was received
    /// from the remote instance
    pub last_heartbeat: Instant,

    /// Contains the version of the remote instance
    pub remote_version: String,

    /// Contains mapping of ids to remote files
    pub files: HashMap<u32, RemoteFile>,

    pub callback_manager: CallbackManager<Msg>,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            last_heartbeat: Instant::now(),
            remote_version: String::default(),
            files: HashMap::default(),
            callback_manager: CallbackManager::default(),
        }
    }
}
