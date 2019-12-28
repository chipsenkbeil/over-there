pub mod transmitter;
pub mod types;

use chrono::prelude::{DateTime, Utc};
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Msg {
    /// ID associated with a request or response
    pub id: u32,

    /// The time at which the message was created
    pub creation_date: DateTime<Utc>,

    /// Content within the message
    content: Box<dyn types::Content>,
}

impl Msg {
    pub fn from_content<T: types::Content + 'static>(content: T) -> Self {
        Self {
            id: random(),
            creation_date: Utc::now(),
            content: Box::new(content),
        }
    }

    pub fn is_content<T: types::Content + 'static>(&self) -> bool {
        self.content.as_any().is::<T>()
    }

    pub fn to_content<T: types::Content + 'static>(&self) -> Option<&T> {
        self.content.as_any().downcast_ref::<T>()
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(slice)
    }
}
