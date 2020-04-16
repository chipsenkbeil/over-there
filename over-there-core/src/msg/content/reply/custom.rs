use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct CustomArgs {
    pub data: Vec<u8>,
}

impl From<Vec<u8>> for CustomArgs {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}
