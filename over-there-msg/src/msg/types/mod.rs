pub mod request;
pub mod response;

use std::any::Any;
use std::fmt::Debug;

#[typetag::serde(tag = "type")]
pub trait Content: Debug {
    fn as_any(&self) -> &dyn Any;
}
