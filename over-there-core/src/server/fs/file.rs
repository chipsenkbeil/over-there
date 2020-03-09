use over_there_derive::Error;
use rand::{rngs::OsRng, RngCore};
use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

#[derive(Debug, Error)]
pub enum LocalFileError {
    SigMismatch,
    IoError(io::Error),
}

/// Represents a result from a local file operation
pub type Result<T> = std::result::Result<T, LocalFileError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LocalFileHandle {
    pub id: u32,
    pub sig: u32,
}

/// Converts handle into its id
impl Into<u32> for LocalFileHandle {
    fn into(self) -> u32 {
        self.id
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LocalFilePermissions {
    pub write: bool,
    pub read: bool,
}

#[derive(Debug)]
pub struct LocalFile {
    /// Represents a unique id with which to lookup the file
    pub(super) id: u32,

    /// Represents a unique signature that acts as a barrier to prevent
    /// unexpected operations on the file from a client with an outdated
    /// understanding of the file
    pub(super) sig: u32,

    /// Represents an underlying file descriptor with which we can read,
    /// write, and perform other operations
    file: File,

    /// Represents the permissions associated with the file when it was opened
    permissions: LocalFilePermissions,

    /// Represents the absolute path to the file; any movement
    /// of the file will result in changing the path
    path: PathBuf,
}

impl LocalFile {
    pub(crate) fn new(
        file: File,
        permissions: LocalFilePermissions,
        path: impl AsRef<Path>,
    ) -> Self {
        let id = OsRng.next_u32();
        let sig = OsRng.next_u32();

        Self {
            id,
            sig,
            file,
            permissions,
            path: path.as_ref().to_path_buf(),
        }
    }

    pub async fn open(
        path: impl AsRef<Path>,
        create: bool,
        write: bool,
        read: bool,
    ) -> io::Result<Self> {
        match OpenOptions::new()
            .create(create)
            .write(write)
            .read(read)
            .open(&path)
            .await
        {
            Ok(file) => {
                let cpath = fs::canonicalize(path).await?;
                let permissions = LocalFilePermissions { write, read };
                Ok(Self::new(file, permissions, cpath))
            }
            Err(x) => Err(x),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn sig(&self) -> u32 {
        self.sig
    }

    pub fn handle(&self) -> LocalFileHandle {
        LocalFileHandle {
            id: self.id,
            sig: self.sig,
        }
    }

    pub fn permissions(&self) -> LocalFilePermissions {
        self.permissions
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Renames a file (if possible) using its underlying path as the origin
    pub async fn rename(
        &mut self,
        sig: u32,
        to: impl AsRef<Path>,
    ) -> Result<u32> {
        if self.sig != sig {
            return Err(LocalFileError::SigMismatch);
        }

        rename(self.path.as_path(), to.as_ref())
            .await
            .map_err(LocalFileError::IoError)?;

        // Update signature to reflect the change and update our internal
        // path so that we can continue to do renames/removals properly
        self.sig = OsRng.next_u32();
        self.path = to.as_ref().to_path_buf();

        Ok(self.sig)
    }

    /// Removes the file (if possible) using its underlying path
    ///
    /// NOTE: If successful, this makes the local file reference no longer
    ///       usable for the majority of its functionality
    pub async fn remove(&mut self, sig: u32) -> Result<()> {
        if self.sig != sig {
            return Err(LocalFileError::SigMismatch);
        }

        remove(self.path.as_path())
            .await
            .map_err(LocalFileError::IoError)?;

        // Update signature to reflect the change
        self.sig = OsRng.next_u32();

        Ok(())
    }

    /// Reads all contents of file from beginning to end
    pub async fn read_all(&mut self, sig: u32) -> Result<Vec<u8>> {
        if self.sig != sig {
            return Err(LocalFileError::SigMismatch);
        }

        let mut buf = Vec::new();

        self.file
            .seek(SeekFrom::Start(0))
            .await
            .map_err(LocalFileError::IoError)?;

        self.file
            .read_to_end(&mut buf)
            .await
            .map_err(LocalFileError::IoError)?;

        Ok(buf)
    }

    /// Overwrites contents of file with provided contents
    pub async fn write_all(&mut self, sig: u32, buf: &[u8]) -> Result<()> {
        if self.sig != sig {
            return Err(LocalFileError::SigMismatch);
        }

        self.file
            .seek(SeekFrom::Start(0))
            .await
            .map_err(LocalFileError::IoError)?;

        self.file
            .set_len(0)
            .await
            .map_err(LocalFileError::IoError)?;

        // Update our sig after we first touch the file so we guarantee
        // that any modification (even partial) is reflected as a change
        self.sig = OsRng.next_u32();

        self.file
            .write_all(buf)
            .await
            .map_err(LocalFileError::IoError)?;

        self.file.flush().await.map_err(LocalFileError::IoError)
    }
}

pub async fn rename(
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
) -> io::Result<()> {
    let metadata = fs::metadata(from.as_ref()).await?;

    if metadata.is_file() {
        fs::rename(from.as_ref(), to.as_ref()).await
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Not a file"))
    }
}

pub async fn remove(path: impl AsRef<Path>) -> io::Result<()> {
    fs::remove_file(path.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom, Write};

    fn create_test_local_file(
        file: std::fs::File,
        path: impl AsRef<Path>,
    ) -> LocalFile {
        LocalFile::new(
            File::from_std(file),
            LocalFilePermissions {
                read: true,
                write: true,
            },
            path,
        )
    }

    #[tokio::test]
    async fn open_should_yield_error_if_file_missing_and_create_false() {
        match LocalFile::open("missingfile", false, true, true).await {
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::NotFound),
            Ok(f) => panic!("Unexpectedly opened missing file: {:?}", f.path()),
        }
    }

    #[tokio::test]
    async fn open_should_return_new_local_file_with_canonical_path() {
        let (path, result) = async {
            let f = tempfile::NamedTempFile::new().unwrap();
            let path = f.path();
            let result = LocalFile::open(path, false, true, true).await;
            (path.to_owned(), result)
        }
        .await;

        match result {
            Ok(f) => assert_eq!(f.path(), path),
            Err(x) => panic!("Failed to open file: {}", x),
        }
    }

    #[tokio::test]
    async fn id_should_return_associated_id() {
        let lf = create_test_local_file(tempfile::tempfile().unwrap(), "");

        assert_eq!(lf.id, lf.id());
    }

    #[tokio::test]
    async fn sig_should_return_associated_sig() {
        let lf = create_test_local_file(tempfile::tempfile().unwrap(), "");

        assert_eq!(lf.sig, lf.sig());
    }

    #[tokio::test]
    async fn handle_should_return_associated_handle_with_id_and_sig() {
        let lf = create_test_local_file(tempfile::tempfile().unwrap(), "");
        let LocalFileHandle { id, sig } = lf.handle();

        assert_eq!(id, lf.id());
        assert_eq!(sig, lf.sig());
    }

    #[tokio::test]
    async fn path_should_return_associated_path() {
        let path_str = "test_cheeseburger";
        let lf =
            create_test_local_file(tempfile::tempfile().unwrap(), path_str);

        assert_eq!(Path::new(path_str), lf.path());
    }

    #[tokio::test]
    async fn read_all_should_yield_error_if_provided_sig_is_different() {
        let mut lf = create_test_local_file(tempfile::tempfile().unwrap(), "");

        let sig = lf.sig();
        match lf.read_all(sig + 1).await {
            Err(LocalFileError::SigMismatch) => {
                assert_eq!(lf.sig(), sig, "Signature changed after error");
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Unexpectedly read file with bad sig"),
        }
    }

    #[tokio::test]
    async fn read_all_should_yield_error_if_file_not_readable() {
        let result = async {
            let f = tempfile::NamedTempFile::new().unwrap();
            let path = f.path();
            LocalFile::open(path, false, true, false).await
        }
        .await;

        let mut lf = result.expect("Failed to open file");
        let sig = lf.sig();

        match lf.read_all(sig).await {
            Err(LocalFileError::IoError(x))
                if x.kind() == io::ErrorKind::Other =>
            {
                assert_eq!(
                    sig,
                    lf.sig(),
                    "Signature was changed when no modification happened"
                );
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Read succeeded unexpectedly"),
        }
    }

    #[tokio::test]
    async fn read_all_should_return_empty_if_file_empty() {
        let mut lf = create_test_local_file(tempfile::tempfile().unwrap(), "");
        let sig = lf.sig();

        match lf.read_all(sig).await {
            Ok(contents) => {
                assert!(
                    contents.is_empty(),
                    "Got non-empty contents from empty file"
                );
                assert_eq!(
                    sig,
                    lf.sig(),
                    "Signature was changed when no modification happened"
                );
            }
            Err(x) => panic!("Unexpected error: {}", x),
        }
    }

    #[tokio::test]
    async fn read_all_should_return_all_file_content_from_start() {
        let contents = b"some contents";

        let mut f = tempfile::tempfile().unwrap();
        f.write_all(contents).unwrap();

        let mut lf = create_test_local_file(f, "");
        let sig = lf.sig();

        match lf.read_all(sig).await {
            Ok(read_contents) => {
                assert_eq!(
                    read_contents, contents,
                    "Read contents was different than expected: {:?}",
                    read_contents
                );
                assert_eq!(
                    sig,
                    lf.sig(),
                    "Signature was changed when no modification happened"
                );
            }
            Err(x) => panic!("Unexpected error: {}", x),
        }
    }

    #[tokio::test]
    async fn write_all_should_yield_error_if_provided_sig_is_different() {
        let mut lf = create_test_local_file(tempfile::tempfile().unwrap(), "");

        let sig = lf.sig();
        match lf.write_all(sig + 1, b"some contents").await {
            Err(LocalFileError::SigMismatch) => {
                assert_eq!(lf.sig(), sig, "Signature changed after error");
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Unexpectedly removed file with bad sig"),
        }
    }

    #[tokio::test]
    async fn write_all_should_yield_error_if_file_not_writeable() {
        let result = async {
            let f = tempfile::NamedTempFile::new().unwrap();
            let path = f.path();
            LocalFile::open(path, false, false, true).await
        }
        .await;

        let mut lf = result.expect("Failed to open file");
        let sig = lf.sig();

        match lf.write_all(sig, b"some content").await {
            Err(LocalFileError::IoError(x))
                if x.kind() == io::ErrorKind::InvalidInput =>
            {
                assert_eq!(
                    sig,
                    lf.sig(),
                    "Signature was changed when no modification happened"
                );
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Write succeeded unexpectedly"),
        }
    }

    #[tokio::test]
    async fn write_all_should_overwrite_file_with_new_contents() {
        let mut f = tempfile::tempfile().unwrap();
        let mut buf = Vec::new();

        // Load the file as a LocalFile
        let mut lf = create_test_local_file(f.try_clone().unwrap(), "");
        let data = vec![1, 2, 3];

        // Put some arbitrary data into the file
        f.write_all(b"some existing data").unwrap();

        // Overwrite the existing data
        let sig = lf.sig();
        lf.write_all(sig, &data).await.unwrap();

        // Verify the data we just wrote
        f.seek(SeekFrom::Start(0)).unwrap();
        f.read_to_end(&mut buf).unwrap();
        assert_ne!(sig, lf.sig(), "Sig was not updated after write");
        assert_eq!(buf, data);

        // Overwrite the existing data (again)
        let sig = lf.sig();
        lf.write_all(sig, &data).await.unwrap();

        // Verify the data we just wrote
        f.seek(SeekFrom::Start(0)).unwrap();
        buf.clear();
        f.read_to_end(&mut buf).unwrap();
        assert_ne!(sig, lf.sig(), "Sig was not updated after write");
        assert_eq!(buf, data);
    }

    #[tokio::test]
    async fn rename_should_yield_error_if_provided_sig_is_different() {
        let mut lf = create_test_local_file(tempfile::tempfile().unwrap(), "");

        let sig = lf.sig();
        match lf.rename(sig + 1, "something_else").await {
            Err(LocalFileError::SigMismatch) => {
                assert_eq!(lf.sig(), sig, "Signature changed after error");
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Unexpectedly renamed file with bad sig"),
        }
    }

    #[tokio::test]
    async fn rename_should_yield_error_if_underlying_path_is_missing() {
        let mut lf = create_test_local_file(tempfile::tempfile().unwrap(), "");

        let sig = lf.sig();
        match lf.rename(sig, "something_else").await {
            Err(LocalFileError::IoError(x))
                if x.kind() == io::ErrorKind::NotFound =>
            {
                assert_eq!(lf.sig(), sig, "Signature changed after error")
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Unexpectedly renamed file with bad path"),
        }
    }

    #[tokio::test]
    async fn rename_should_yield_error_if_new_name_on_different_mount_point() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let path = f.path();

        let mut lf =
            create_test_local_file(f.as_file().try_clone().unwrap(), path);

        // NOTE: Renaming when using temp file seems to trigger this, so using
        //       it as a test case
        let sig = lf.sig();
        match lf.rename(sig, "renamed_file").await {
            Err(_) => {
                assert_eq!(lf.sig(), sig, "Signature changed after error")
            }
            Ok(_) => panic!("Unexpectedly suceeded in rename: {:?}", lf.path()),
        }
    }

    #[tokio::test]
    async fn rename_should_move_file_to_another_location_by_path() {
        let mut lf = LocalFile::open("file_to_rename", true, true, true)
            .await
            .expect("Failed to open");
        let sig = lf.sig();

        // Do rename and verify that the file at the new path exists
        assert!(
            fs::read("renamed_file").await.is_err(),
            "File already exists at rename path"
        );
        let new_sig = lf
            .rename(sig, "renamed_file")
            .await
            .expect("Failed to rename");
        assert!(
            fs::read("renamed_file").await.is_ok(),
            "File did not get renamed to new path"
        );
        fs::remove_file("renamed_file")
            .await
            .expect("Failed to clean up file");

        // Verify signature changed
        assert_ne!(new_sig, sig);
    }

    #[tokio::test]
    async fn remove_should_yield_error_if_provided_sig_is_different() {
        let mut lf = create_test_local_file(tempfile::tempfile().unwrap(), "");

        let sig = lf.sig();
        match lf.remove(sig + 1).await {
            Err(LocalFileError::SigMismatch) => {
                assert_eq!(lf.sig(), sig, "Signature changed after error");
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Unexpectedly removed file with bad sig"),
        }
    }

    #[tokio::test]
    async fn remove_should_yield_error_if_underlying_path_is_missing() {
        let mut lf = create_test_local_file(tempfile::tempfile().unwrap(), "");

        let sig = lf.sig();
        match lf.remove(sig).await {
            Err(LocalFileError::IoError(x))
                if x.kind() == io::ErrorKind::NotFound =>
            {
                assert_eq!(lf.sig(), sig, "Signature changed after error");
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Unexpectedly removed file with bad path"),
        }
    }

    #[tokio::test]
    async fn remove_should_remove_the_underlying_file_by_path() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let path = f.path();

        let mut lf =
            create_test_local_file(f.as_file().try_clone().unwrap(), path);

        let sig = lf.sig();

        // Do remove and verify that the file at path is gone
        assert!(fs::read(path).await.is_ok(), "File already missing at path");
        lf.remove(sig).await.expect("Failed to remove file");
        assert!(fs::read(path).await.is_err(), "File still exists at path");
        assert_ne!(sig, lf.sig(), "Signature was not updated");
    }
}
