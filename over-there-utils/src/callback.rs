use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

pub type Callback<T> = dyn FnOnce(&T) + Send;

pub struct CallbackManager<T> {
    /// Contains callback functions to invoke when a
    /// response is received for a msg with a specific id
    callbacks: HashMap<u32, Box<Callback<T>>>,
}

impl<T> CallbackManager<T> {
    pub fn add_callback(&mut self, id: u32, callback: impl FnOnce(&T) + Send + 'static) {
        self.callbacks.insert(id, Box::new(callback));
    }

    pub fn take_callback(&mut self, id: u32) -> Option<Box<Callback<T>>> {
        self.callbacks.remove(&id)
    }
}

impl<T> Default for CallbackManager<T> {
    fn default() -> Self {
        Self {
            callbacks: HashMap::default(),
        }
    }
}

impl<T> Debug for CallbackManager<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CallbackManager {{ callbacks: {:?} }}",
            self.callbacks.keys(),
        )
    }
}
