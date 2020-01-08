use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CustomRequest {
    /// Key-value map for custom requests
    CustomRequest { data: HashMap<String, String> },
}
