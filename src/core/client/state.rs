use super::file::RemoteFile;
use crate::core::msg::content::Reply;
use crate::utils::CallbackManager;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug)]
pub struct ClientState {
    /// Contains last time when client received response from server
    pub last_contact: Instant,

    /// Contains the version of the remote instance
    pub remote_version: String,

    /// Contains mapping of ids to remote files
    pub files: HashMap<u32, RemoteFile>,

    pub callback_manager: CallbackManager<Reply>,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            last_contact: Instant::now(),
            remote_version: String::default(),
            files: HashMap::default(),
            callback_manager: CallbackManager::default(),
        }
    }
}
