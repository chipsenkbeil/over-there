#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    pub(crate) id: u32,
    pub(crate) sig: u32,
    pub(crate) path: String,
}

impl RemoteFile {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}
