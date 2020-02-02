use std::process::Child;

#[derive(Debug)]
pub struct LocalProc {
    pub(crate) id: u32,
    pub child: Child,
}
