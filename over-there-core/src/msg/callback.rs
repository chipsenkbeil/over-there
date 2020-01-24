use super::Msg;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

pub type Callback = dyn FnMut(&Msg);

pub struct CallbackManager {
    /// Contains callback functions to invoke when a
    /// response is received for a msg with a specific id
    callbacks: HashMap<u32, Box<Callback>>,
}

impl CallbackManager {
    pub fn add_callback(&mut self, id: u32, callback: impl FnMut(&Msg) + 'static) {
        self.callbacks.insert(id, Box::new(callback));
    }

    pub fn take_callback(&mut self, id: u32) -> Option<Box<Callback>> {
        self.callbacks.remove(&id)
    }
}

impl Default for CallbackManager {
    fn default() -> Self {
        Self {
            callbacks: HashMap::default(),
        }
    }
}

impl Debug for CallbackManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CallbackManager {{ callbacks: {:?} }}",
            self.callbacks.keys(),
        )
    }
}
