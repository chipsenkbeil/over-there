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

    /// Default file ttl (time since last touched) before until closing (2.5 min)
    pub const DEFAULT_FILE_TTL: Duration = Duration::from_secs(60 * 2.5 as u64);

    /// Default proc ttl (time since last touched) before until killing (5 min)
    pub const DEFAULT_PROC_TTL: Duration = Duration::from_secs(60 * 5);

    /// Default proc ttl (time since last touched) since a proc has exited
    /// before removing from queriable state (30 sec)
    pub const DEFAULT_DEAD_PROC_TTL: Duration = Duration::from_secs(30);
}

#[derive(Debug)]
pub struct ServerState {
    /// Connections server has with clients and last time each client
    /// communicated with the server
    pub conns: Mutex<HashMap<SocketAddr, Instant>>,

    /// Mapping of file id -> file on same machine as server
    pub fs_manager: Mutex<FileSystemManager>,
    pub(super) file_ids: Mutex<HashSet<TtlValue<u32>>>,
    file_ttl: Duration,

    /// Mapping of proc id -> proc on same machine as server
    pub procs: Mutex<HashMap<u32, LocalProc>>,
    pub(super) proc_ids: Mutex<HashSet<TtlValue<u32>>>,
    proc_ttl: Duration,
    pub(crate) dead_proc_ttl: Duration,

    /// Indicator of whether or not the server is running, used to signal
    /// to looping handlers that it is time to shut down if false
    running: AtomicBool,
}

impl ServerState {
    pub fn new(
        file_ttl: Duration,
        proc_ttl: Duration,
        dead_proc_ttl: Duration,
    ) -> Self {
        Self {
            conns: Mutex::new(HashMap::default()),
            fs_manager: Mutex::new(FileSystemManager::default()),
            file_ids: Mutex::new(HashSet::default()),
            file_ttl,
            procs: Mutex::new(HashMap::default()),
            proc_ids: Mutex::new(HashSet::default()),
            proc_ttl,
            dead_proc_ttl,
            running: AtomicBool::new(true),
        }
    }

    /// Creates or updates an internal TTL for a file with `id` using the
    /// state-configured TTL as the max untouched lifetime
    pub async fn touch_file_id(&self, id: u32) {
        self.touch_file_id_with_ttl(id, self.file_ttl).await;
    }

    /// Creates or updates an internal TTL for a file with `id` using the
    /// given `ttl` as the max untouched lifetime
    pub async fn touch_file_id_with_ttl(&self, id: u32, ttl: Duration) {
        self.file_ids.lock().await.replace(TtlValue::new(id, ttl));
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

    /// Creates or updates an internal TTL for a proc with `id` using the
    /// state-configured TTL as the max untouched lifetime
    pub async fn touch_proc_id(&self, id: u32) {
        self.touch_proc_id_with_ttl(id, self.proc_ttl).await;
    }

    /// Creates or updates an internal TTL for a proc with `id` using the
    /// given `ttl` as the max untouched lifetime
    pub async fn touch_proc_id_with_ttl(&self, id: u32, ttl: Duration) {
        self.proc_ids.lock().await.replace(TtlValue::new(id, ttl));
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
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Acquires debug-related information for each part of state,
    /// which requires locking each component
    pub(crate) async fn internal_debug(&self) -> String {
        format!(
            "Conns: {:#?}
            FS Manager: {:#?}
            Files IDs: {:#?}
            File Untouched TTL: {:?}
            Procs: {:#?}
            Proc IDs: {:#?}
            Proc Untouched TTL: {:?}
            Dead Proc Untouched TTL: {:?}",
            self.conns.lock().await,
            self.fs_manager.lock().await,
            self.file_ids.lock().await,
            self.file_ttl,
            self.procs.lock().await,
            self.proc_ids.lock().await,
            self.proc_ttl,
            self.dead_proc_ttl,
        )
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new(
            constants::DEFAULT_FILE_TTL,
            constants::DEFAULT_PROC_TTL,
            constants::DEFAULT_DEAD_PROC_TTL,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    use tokio::process::Command;

    #[tokio::test]
    async fn touch_file_id_should_produce_a_new_id_if_never_touched() {
        let state = ServerState::default();

        assert!(!state.file_ids.lock().await.contains(&From::from(1)));

        state.touch_file_id(1).await;

        let ids = state.file_ids.lock().await;
        let id = ids.get(&From::from(1)).expect("File id missing");

        assert_eq!(id.ttl(), &state.file_ttl);
    }

    #[tokio::test]
    async fn touch_file_id_should_update_an_existing_id_if_touched() {
        let state = ServerState::default();

        state.touch_file_id(1).await;

        let last_touched = {
            let ids = state.file_ids.lock().await;
            let id = ids.get(&From::from(1)).expect("File id missing");
            *id.last_touched()
        };

        state.touch_file_id(1).await;

        let ids = state.file_ids.lock().await;
        let id = ids.get(&From::from(1)).expect("File id missing");

        assert!(
            id.last_touched() > &last_touched,
            "Did not update touch time"
        );
    }

    #[tokio::test]
    async fn touch_file_id_with_ttl_should_produce_a_new_id_if_never_touched() {
        let state = ServerState::default();

        assert!(!state.file_ids.lock().await.contains(&From::from(1)));

        state
            .touch_file_id_with_ttl(1, Duration::new(999, 111))
            .await;

        let ids = state.file_ids.lock().await;
        let id = ids.get(&From::from(1)).expect("file id missing");

        assert_eq!(id.ttl(), &Duration::new(999, 111));
    }

    #[tokio::test]
    async fn touch_file_id_with_ttl_should_update_an_existing_id_if_touched() {
        let state = ServerState::default();

        state
            .touch_file_id_with_ttl(1, Duration::new(222, 333))
            .await;

        let last_touched = {
            let ids = state.file_ids.lock().await;
            let id = ids.get(&From::from(1)).expect("File id missing");
            *id.last_touched()
        };

        state
            .touch_file_id_with_ttl(1, Duration::new(999, 111))
            .await;

        let ids = state.file_ids.lock().await;
        let id = ids.get(&From::from(1)).expect("File id missing");

        assert!(
            id.last_touched() > &last_touched,
            "Did not update touch time"
        );
        assert_eq!(id.ttl(), &Duration::new(999, 111));
    }

    #[tokio::test]
    async fn remove_file_id_should_remove_id_without_closing_file_if_open() {
        let tempdir = tempfile::tempdir().expect("Failed to create temp dir");
        let state = ServerState::default();

        // Open a temporary file and add the id to our list
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(
                tempdir.as_ref().join("test-file").as_path(),
                true,
                true,
                true,
            )
            .await
            .expect("Failed to open test file");

        state.touch_file_id(handle.id).await;

        // Now remove the file id and verify it is still open in the manager
        state.remove_file_id(handle.id).await;
        assert!(
            !state.file_ids.lock().await.contains(&From::from(handle.id)),
            "ID was unexpectedly not removed from list"
        );
        assert!(
            state.fs_manager.lock().await.exists(handle.id),
            "File was unexpectedly removed from manager"
        );
    }

    #[tokio::test]
    async fn evict_files_should_close_any_file_that_has_expired() {
        let tempdir = tempfile::tempdir().expect("Failed to create temp dir");
        let state = ServerState::default();

        // Open two temporary files, one of which we'll mark with a low
        // TTL and the other a high TTL
        let handle_1 = state
            .fs_manager
            .lock()
            .await
            .open_file(
                tempdir.as_ref().join("test-file-1").as_path(),
                true,
                true,
                true,
            )
            .await
            .expect("Failed to open test file 1");
        let handle_2 = state
            .fs_manager
            .lock()
            .await
            .open_file(
                tempdir.as_ref().join("test-file-2").as_path(),
                true,
                true,
                true,
            )
            .await
            .expect("Failed to open test file 2");

        // File 1 will be a short TTL
        state
            .touch_file_id_with_ttl(handle_1.id, Duration::new(0, 0))
            .await;

        // File 2 will be a long TTL
        state
            .touch_file_id_with_ttl(handle_2.id, Duration::from_secs(60))
            .await;

        // Now evict the files that have expired and validate that only the
        // short TTL file has been evicted
        state.evict_files().await;

        assert!(
            !state
                .file_ids
                .lock()
                .await
                .contains(&From::from(handle_1.id)),
            "File 1 id was unexpectedly not removed from list"
        );
        assert!(
            state
                .file_ids
                .lock()
                .await
                .contains(&From::from(handle_2.id)),
            "File 2 id was unexpectedly removed from list"
        );
        assert!(
            !state.fs_manager.lock().await.exists(handle_1.id),
            "File 1 was unexpectedly not removed from manager"
        );
        assert!(
            state.fs_manager.lock().await.exists(handle_2.id),
            "File 2 was unexpectedly removed from manager"
        );
    }

    #[tokio::test]
    async fn touch_proc_id_should_produce_a_new_id_if_never_touched() {
        let state = ServerState::default();

        assert!(!state.proc_ids.lock().await.contains(&From::from(1)));

        state.touch_proc_id(1).await;

        let ids = state.proc_ids.lock().await;
        let id = ids.get(&From::from(1)).expect("Proc id missing");

        assert_eq!(id.ttl(), &state.proc_ttl);
    }

    #[tokio::test]
    async fn touch_proc_id_should_update_an_existing_id_if_touched() {
        let state = ServerState::default();

        state.touch_proc_id(1).await;

        let last_touched = {
            let ids = state.proc_ids.lock().await;
            let id = ids.get(&From::from(1)).expect("Proc id missing");
            *id.last_touched()
        };

        state.touch_proc_id(1).await;

        let ids = state.proc_ids.lock().await;
        let id = ids.get(&From::from(1)).expect("Proc id missing");

        assert!(
            id.last_touched() > &last_touched,
            "Did not update touch time"
        );
    }

    #[tokio::test]
    async fn touch_proc_id_with_ttl_should_produce_a_new_id_if_never_touched() {
        let state = ServerState::default();

        assert!(!state.proc_ids.lock().await.contains(&From::from(1)));

        state
            .touch_proc_id_with_ttl(1, Duration::new(999, 111))
            .await;

        let ids = state.proc_ids.lock().await;
        let id = ids.get(&From::from(1)).expect("Proc id missing");

        assert_eq!(id.ttl(), &Duration::new(999, 111));
    }

    #[tokio::test]
    async fn touch_proc_id_with_ttl_should_update_an_existing_id_if_touched() {
        let state = ServerState::default();

        state
            .touch_proc_id_with_ttl(1, Duration::new(222, 333))
            .await;

        let last_touched = {
            let ids = state.proc_ids.lock().await;
            let id = ids.get(&From::from(1)).expect("Proc id missing");
            *id.last_touched()
        };

        state
            .touch_proc_id_with_ttl(1, Duration::new(999, 111))
            .await;

        let ids = state.proc_ids.lock().await;
        let id = ids.get(&From::from(1)).expect("Proc id missing");

        assert!(
            id.last_touched() > &last_touched,
            "Did not update touch time"
        );
        assert_eq!(id.ttl(), &Duration::new(999, 111));
    }

    #[tokio::test]
    async fn remove_proc_id_should_remove_id_without_killing_proc_if_running() {
        let state = ServerState::default();

        // Spawn a process that will run for awhile
        let child = Command::new("sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("Failed to spawn child process");

        let local_proc = LocalProc::new(child).spawn();
        let id = local_proc.id();

        // Add the process to our internal mapping
        state.procs.lock().await.insert(id, local_proc);

        // Add the id, then remove it
        state.touch_proc_id(id).await;
        state.remove_proc_id(id).await;

        // Verify that the id has been removed from our list, but not the
        // map of ids to procs
        assert!(
            !state.proc_ids.lock().await.contains(&From::from(id)),
            "ID was unexpectedly not removed from list"
        );
        assert!(
            state.procs.lock().await.contains_key(&id),
            "Proc was unexpectedly removed from map"
        );

        // Verify that the process has not exited/been killed
        let mut procs = state.procs.lock().await;
        match procs
            .get_mut(&id)
            .expect("Missing proc in map")
            .exit_status()
            .await
        {
            None => (),
            Some(x) => panic!("Unexpected content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn evict_procs_should_close_any_proc_that_has_expired() {
        let state = ServerState::default();

        // Spawn a process that will run for awhile with low TTL and another
        // process that will run for awhile with high TTL
        let child_1 = Command::new("sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("Failed to spawn child process 1");
        let child_2 = Command::new("sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("Failed to spawn child process 2");

        // Add the processes to our internal mapping
        let local_proc_1 = LocalProc::new(child_1).spawn();
        let id_1 = local_proc_1.id();
        state.procs.lock().await.insert(id_1, local_proc_1);

        let local_proc_2 = LocalProc::new(child_2).spawn();
        let id_2 = local_proc_2.id();
        state.procs.lock().await.insert(id_2, local_proc_2);

        // Proc 1 will be a short TTL
        state
            .touch_proc_id_with_ttl(id_1, Duration::new(0, 0))
            .await;

        // Proc 2 will be a long TTL
        state
            .touch_proc_id_with_ttl(id_2, Duration::from_secs(60))
            .await;

        // Evict expired procs
        state.evict_procs().await;

        // Verify that proc 1 has been removed while proc 2 has not
        assert!(
            !state.proc_ids.lock().await.contains(&From::from(id_1)),
            "Proc 1 was unexpectedly not removed from list"
        );
        assert!(
            state.proc_ids.lock().await.contains(&From::from(id_2)),
            "Proc 2 was unexpectedly removed from list"
        );
        assert!(
            !state.procs.lock().await.contains_key(&id_1),
            "Proc 1 was unexpectedly not removed from map"
        );
        assert!(
            state.procs.lock().await.contains_key(&id_2),
            "Proc 2 was unexpectedly removed from map"
        );

        // Verify that proc 2 has not exited/been killed
        let mut procs = state.procs.lock().await;
        match procs
            .get_mut(&id_2)
            .expect("Missing proc 2 in map")
            .exit_status()
            .await
        {
            None => (),
            Some(x) => panic!("Unexpected content: {:?}", x),
        }
    }
}
