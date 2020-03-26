use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ErrorArgs {
    pub msg: String,
}

impl From<Box<dyn std::error::Error>> for ErrorArgs {
    fn from(x: Box<dyn std::error::Error>) -> Self {
        Self {
            msg: format!("{}", x),
        }
    }
}
