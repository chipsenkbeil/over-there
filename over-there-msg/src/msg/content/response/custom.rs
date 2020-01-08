use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CustomResponse {
    /// Key-value map for custom responses
    CustomResponse { data: HashMap<String, String> },
}
