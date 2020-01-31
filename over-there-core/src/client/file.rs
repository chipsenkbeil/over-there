#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    pub id: u32,
    pub(crate) sig: u32,
    pub path: String,
}
