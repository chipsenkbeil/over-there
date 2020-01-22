use std::time::Instant;

pub struct State {
    pub last_heartbeat: Instant,
    pub remote_version: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_heartbeat: Instant::now(),
            remote_version: String::default(),
        }
    }
}
