use super::{fs::FileSystemManager, proc::LocalProc};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ServerState {
    /// Connections server has with clients and last time each client
    /// communicated with the server
    pub conns: Mutex<HashMap<SocketAddr, Instant>>,

    /// Mapping of file id -> file on same machine as server
    pub fs_manager: Mutex<FileSystemManager>,
    pub conn_files: Mutex<HashMap<SocketAddr, Vec<u32>>>,

    /// Mapping of proc id -> proc on same machine as server
    pub procs: Mutex<HashMap<u32, LocalProc>>,
    pub conn_procs: Mutex<HashMap<SocketAddr, Vec<u32>>>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            conns: Mutex::new(HashMap::default()),
            fs_manager: Mutex::new(FileSystemManager::default()),
            conn_files: Mutex::new(HashMap::default()),
            procs: Mutex::new(HashMap::default()),
            conn_procs: Mutex::new(HashMap::default()),
        }
    }
}
