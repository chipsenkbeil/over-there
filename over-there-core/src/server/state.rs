use super::{file::LocalFile, proc::LocalProc};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ServerState {
    pub files: HashMap<u32, LocalFile>,
    pub procs: HashMap<u32, LocalProc>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            files: HashMap::default(),
            procs: HashMap::default(),
        }
    }
}
