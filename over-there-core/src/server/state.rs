use super::file::LocalFile;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ServerState {
    pub files: HashMap<u32, LocalFile>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            files: HashMap::default(),
        }
    }
}
