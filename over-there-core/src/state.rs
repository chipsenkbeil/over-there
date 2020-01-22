use std::time::Instant;

pub struct State {
    pub last_heartbeat: Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_heartbeat: Instant::now(),
        }
    }
}
