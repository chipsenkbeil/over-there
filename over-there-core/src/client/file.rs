#[derive(Debug)]
pub struct RemoteFile {
    pub(crate) id: u32,
    pub(crate) sig: u32,
    pub path: String,
}
