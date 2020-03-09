use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InternalDebugArgs {
    /// Used to provide input to some service when querying internal state
    pub input: Vec<u8>,

    /// Used as output when replying to a service that queried internal state
    pub output: Vec<u8>,
}
