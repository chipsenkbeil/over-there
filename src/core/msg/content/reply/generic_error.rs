use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct GenericErrorArgs {
    pub msg: String,
}

impl crate::core::SchemaInfo for GenericErrorArgs {}

impl ToString for GenericErrorArgs {
    fn to_string(&self) -> String {
        self.msg.clone()
    }
}

impl From<Box<dyn std::error::Error>> for GenericErrorArgs {
    fn from(x: Box<dyn std::error::Error>) -> Self {
        Self {
            msg: format!("{}", x),
        }
    }
}

impl From<String> for GenericErrorArgs {
    fn from(text: String) -> Self {
        Self { msg: text }
    }
}

impl From<&str> for GenericErrorArgs {
    fn from(text: &str) -> Self {
        Self::from(String::from(text))
    }
}
