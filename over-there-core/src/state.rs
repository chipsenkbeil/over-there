use super::msg::Msg;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::time::Instant;

pub type Callback = dyn FnMut(&Msg);

pub struct State {
    /// Contains the time when the last heartbeat was received
    /// from the remote instance
    pub last_heartbeat: Instant,

    /// Contains the version of the remote instance
    pub remote_version: String,

    /// Contains callback functions to invoke when a
    /// response is received for a msg with a specific id
    callbacks: HashMap<u32, Box<Callback>>,
}

impl State {
    pub fn add_callback(&mut self, id: u32, callback: impl FnMut(&Msg) + 'static) {
        self.callbacks.insert(id, Box::new(callback));
    }

    pub fn take_callback(&mut self, id: u32) -> Option<Box<Callback>> {
        self.callbacks.remove(&id)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_heartbeat: Instant::now(),
            remote_version: String::default(),
            callbacks: HashMap::default(),
        }
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "State {{ last_heartbeat: {:?}, remote_version: {}, callbacks: {:?} }}",
            self.last_heartbeat,
            self.remote_version,
            self.callbacks.keys(),
        )
    }
}
