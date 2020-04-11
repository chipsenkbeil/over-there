use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct InternalDebugArgs {
    pub input: Vec<u8>,
}
