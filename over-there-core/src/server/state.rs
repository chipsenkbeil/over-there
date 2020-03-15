use super::{fs::FileSystemManager, proc::LocalProc};
use log::error;
use over_there_utils::TtlValue;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub mod constants {
    use std::time::Duration;

    /// Default file ttl (time since last touched) before until closing
    pub const DEFAULT_FILE_TTL: Duration = Duration::from_secs(60 * 60);

    /// Default proc ttl (time since last touched) before until killing
    pub const DEFAULT_PROC_TTL: Duration = Duration::from_secs(60 * 60);
}

#[derive(Debug)]
pub struct ServerState {
    /// Connections server has with clients and last time each client
    /// communicated with the server
    pub conns: Mutex<HashMap<SocketAddr, Instant>>,

    /// Mapping of file id -> file on same machine as server
    pub fs_manager: Mutex<FileSystemManager>,
    file_ids: Mutex<HashSet<TtlValue<u32>>>,
    file_ttl: Duration,

    /// Mapping of proc id -> proc on same machine as server
    pub procs: Mutex<HashMap<u32, LocalProc>>,
    proc_ids: Mutex<HashSet<TtlValue<u32>>>,
    proc_ttl: Duration,

    /// Indicator of whether or not the server is running, used to signal
    /// to looping handlers that it is time to shut down if false
    running: AtomicBool,
}

impl ServerState {
    /// Produces new state where the server's fs-based operations are locked
    /// to the specified `root`
    pub fn new(file_ttl: Duration, proc_ttl: Duration) -> Self {
        Self {
            conns: Mutex::new(HashMap::default()),
            fs_manager: Mutex::new(FileSystemManager::default()),
            file_ids: Mutex::new(HashSet::default()),
            file_ttl,
            procs: Mutex::new(HashMap::default()),
            proc_ids: Mutex::new(HashSet::default()),
            proc_ttl,
            running: AtomicBool::new(true),
        }
    }

    /// Creates or updates an internal TTL for a file with associated id
    pub async fn touch_file_id(&self, id: u32) {
        self.file_ids
            .lock()
            .await
            .replace(TtlValue::new(id, self.file_ttl));
    }

    /// Removes id associated with an open file, used for internal TTL tracking
    pub async fn remove_file_id(&self, id: u32) {
        self.file_ids.lock().await.remove(&TtlValue::from(id));
    }

    /// Evicts any files that have not been touched in TTL or longer time,
    /// removing them using the associated file manager
    pub async fn evict_files(&self) {
        let mut fsm = self.fs_manager.lock().await;
        self.file_ids.lock().await.retain(|v| {
            let expired = v.has_expired();

            if expired {
                let handle = fsm.get(**v).map(|f| f.handle());

                if let Some(h) = handle {
                    if let Err(x) = fsm.close_file(h) {
                        error!("Failed to evict file {}: {}", **v, x);
                    }
                }
            }

            !expired
        });
    }

    /// Creates or updates an internal TTL for a proc with associated id
    pub async fn touch_proc_id(&self, id: u32) {
        self.proc_ids
            .lock()
            .await
            .replace(TtlValue::new(id, self.proc_ttl));
    }

    /// Removes id associated with a proc, used for internal TTL tracking
    pub async fn remove_proc_id(&self, id: u32) {
        self.proc_ids.lock().await.remove(&TtlValue::from(id));
    }

    /// Evicts any proc that have not been touched in TTL or longer time,
    /// removing them by killing them
    pub async fn evict_procs(&self) {
        let mut proc_map = self.procs.lock().await;
        self.proc_ids.lock().await.retain(|v| {
            let expired = v.has_expired();

            if expired {
                if let Some(mut proc) = proc_map.remove(&**v) {
                    if let Err(x) = proc.kill() {
                        error!("Failed to kill proc {}: {}", **v, x);
                    }
                }
            }

            !expired
        });
    }

    /// Reports the status of the server, used by looping tasks to know whether
    /// to continue running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Updates the state to reflect that the server is shutting down
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Acquires debug-related information for each part of state,
    /// which requires locking each component
    pub(crate) async fn internal_debug(&self) -> String {
        format!(
            "Conns: {:#?}
            FS Manager: {:#?}
            Files IDs: {:#?}
            Procs: {:#?}
            Proc IDs: {:#?}",
            self.conns.lock().await,
            self.fs_manager.lock().await,
            self.file_ids.lock().await,
            self.procs.lock().await,
            self.proc_ids.lock().await,
        )
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new(constants::DEFAULT_FILE_TTL, constants::DEFAULT_PROC_TTL)
    }
}
