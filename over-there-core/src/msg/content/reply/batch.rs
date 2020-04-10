use super::Content;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct BatchArgs {
    pub results: Vec<Content>,
}
