use crate::Reply;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct BatchArgs {
    pub results: Vec<Reply>,
}

impl From<Vec<Reply>> for BatchArgs {
    fn from(results: Vec<Reply>) -> Self {
        Self { results }
    }
}
