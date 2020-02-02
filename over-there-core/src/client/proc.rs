#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProc {
    pub(crate) id: u32,
}

impl RemoteProc {
    pub fn id(&self) -> u32 {
        self.id
    }
}
