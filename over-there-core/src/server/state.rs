use super::{file::LocalFile, proc::LocalProc};
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ServerState {
    pub files: Mutex<HashMap<u32, LocalFile>>,
    pub procs: Mutex<HashMap<u32, LocalProc>>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            files: Mutex::new(HashMap::default()),
            procs: Mutex::new(HashMap::default()),
        }
    }
}
