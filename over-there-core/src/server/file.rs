use tokio::fs::File;

#[derive(Debug)]
pub struct LocalFile {
    pub(crate) id: u32,
    pub(crate) sig: u32,
    pub file: File,
}
