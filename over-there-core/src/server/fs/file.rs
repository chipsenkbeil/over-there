use over_there_derive::Error;
use rand::{rngs::OsRng, RngCore};
use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

#[derive(Debug, Error)]
pub enum LocalFileWriteError {
    SigMismatch,
    IoError(LocalFileWriteIoError),
}

#[derive(Debug, Error)]
pub enum LocalFileWriteIoError {
    SeekError(io::Error),
    SetLenError(io::Error),
    WriteAllError(io::Error),
    FlushError(io::Error),
}

impl Into<io::Error> for LocalFileWriteIoError {
    fn into(self: Self) -> io::Error {
        match self {
            Self::SeekError(x) => x,
            Self::SetLenError(x) => x,
            Self::WriteAllError(x) => x,
            Self::FlushError(x) => x,
        }
    }
}

#[derive(Debug, Error)]
pub enum LocalFileReadError {
    SigMismatch,
    IoError(LocalFileReadIoError),
}

#[derive(Debug, Error)]
pub enum LocalFileReadIoError {
    SeekError(io::Error),
    ReadToEndError(io::Error),
}

impl Into<io::Error> for LocalFileReadIoError {
    fn into(self: Self) -> io::Error {
        match self {
            Self::SeekError(x) => x,
            Self::ReadToEndError(x) => x,
        }
    }
}

#[derive(Debug, Error)]
pub enum LocalFileRenameError {
    SigMismatch,
    IoError(io::Error),
}

#[derive(Debug, Error)]
pub enum LocalFileRemoveError {
    SigMismatch(LocalFile),
    IoError(io::Error, LocalFile),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LocalFileHandle {
    pub id: u32,
    pub sig: u32,
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

    /// Represents the absolute path to the file; any movement
    /// of the file will result in changing the path
    path: PathBuf,
}

impl LocalFile {
    pub(crate) fn new(file: File, path: impl AsRef<Path>) -> Self {
        let id = OsRng.next_u32();
        let sig = OsRng.next_u32();

        Self {
            id,
            sig,
            file,
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
                Ok(Self::new(file, cpath))
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

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Only to be invoked when a path was changed elsewhere such that this
    /// file can update its internal path reference.
    ///
    /// Note that this does not require a sig to apply the change and will
    /// also not update the associated file's sig as this is considered a
    /// background change that should not affect the file descriptor in
    /// any way.
    pub(crate) fn apply_path_changed(
        &mut self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) {
        if let Ok(path) = self.path.strip_prefix(from) {
            self.path = to.as_ref().join(path);
        }
    }

    /// Renames a file (if possible) using its underlying path as the origin
    pub async fn rename(
        &mut self,
        sig: u32,
        to: impl AsRef<Path>,
    ) -> Result<(), LocalFileRenameError> {
        if self.sig != sig {
            return Err(LocalFileRenameError::SigMismatch);
        }

        fs::rename(self.path.as_path(), to.as_ref())
            .await
            .map_err(LocalFileRenameError::IoError)?;

        // Update signature to reflect the change and update our internal
        // path so that we can continue to do renames/removals properly
        self.sig = OsRng.next_u32();
        self.path = to.as_ref().to_path_buf();

        Ok(())
    }

    /// Removes the file (if possible) using its underlying path
    pub async fn remove(self, sig: u32) -> Result<(), LocalFileRemoveError> {
        if self.sig != sig {
            return Err(LocalFileRemoveError::SigMismatch(self));
        }

        fs::remove_file(self.path.as_path())
            .await
            .map_err(|x| LocalFileRemoveError::IoError(x, self))?;

        Ok(())
    }

    /// Reads all contents of file from beginning to end
    pub async fn read_all(
        &mut self,
        sig: u32,
    ) -> Result<Vec<u8>, LocalFileReadError> {
        if self.sig != sig {
            return Err(LocalFileReadError::SigMismatch);
        }

        let mut buf = Vec::new();

        self.file
            .seek(SeekFrom::Start(0))
            .await
            .map_err(LocalFileReadIoError::SeekError)
            .map_err(LocalFileReadError::IoError)?;

        self.file
            .read_to_end(&mut buf)
            .await
            .map_err(LocalFileReadIoError::ReadToEndError)
            .map_err(LocalFileReadError::IoError)?;

        Ok(buf)
    }

    /// Overwrites contents of file with provided contents
    pub async fn write_all(
        &mut self,
        sig: u32,
        buf: &[u8],
    ) -> Result<(), LocalFileWriteError> {
        if self.sig != sig {
            return Err(LocalFileWriteError::SigMismatch);
        }

        self.file
            .seek(SeekFrom::Start(0))
            .await
            .map_err(LocalFileWriteIoError::SeekError)
            .map_err(LocalFileWriteError::IoError)?;

        self.file
            .set_len(0)
            .await
            .map_err(LocalFileWriteIoError::SetLenError)
            .map_err(LocalFileWriteError::IoError)?;

        // Update our sig after we first touch the file so we guarantee
        // that any modification (even partial) is reflected as a change
        self.sig = OsRng.next_u32();

        self.file
            .write_all(buf)
            .await
            .map_err(LocalFileWriteIoError::WriteAllError)
            .map_err(LocalFileWriteError::IoError)?;

        self.file
            .flush()
            .await
            .map_err(LocalFileWriteIoError::FlushError)
            .map_err(LocalFileWriteError::IoError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom, Write};

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
        let lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");

        assert_eq!(lf.id, lf.id());
    }

    #[tokio::test]
    async fn sig_should_return_associated_sig() {
        let lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");

        assert_eq!(lf.sig, lf.sig());
    }

    #[tokio::test]
    async fn handle_should_return_associated_handle_with_id_and_sig() {
        let lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");
        let LocalFileHandle { id, sig } = lf.handle();

        assert_eq!(id, lf.id());
        assert_eq!(sig, lf.sig());
    }

    #[tokio::test]
    async fn path_should_return_associated_path() {
        let path_str = "test_cheeseburger";
        let lf = LocalFile::new(
            File::from_std(tempfile::tempfile().unwrap()),
            path_str,
        );

        assert_eq!(Path::new(path_str), lf.path());
    }

    #[tokio::test]
    async fn read_all_should_yield_error_if_provided_sig_is_different() {
        let mut lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");

        let sig = lf.sig();
        match lf.read_all(sig + 1).await {
            Err(LocalFileReadError::SigMismatch) => {
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
            Err(LocalFileReadError::IoError(
                LocalFileReadIoError::ReadToEndError(_),
            )) => {
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
        let mut lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");
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

        let mut lf = LocalFile::new(File::from_std(f), "");
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
        let mut lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");

        let sig = lf.sig();
        match lf.write_all(sig + 1, b"some contents").await {
            Err(LocalFileWriteError::SigMismatch) => {
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
            Err(LocalFileWriteError::IoError(
                LocalFileWriteIoError::SetLenError(_),
            )) => {
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
        let mut lf = LocalFile::new(File::from_std(f.try_clone().unwrap()), "");
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
        let mut lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");

        let sig = lf.sig();
        match lf.rename(sig + 1, "something_else").await {
            Err(LocalFileRenameError::SigMismatch) => {
                assert_eq!(lf.sig(), sig, "Signature changed after error");
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Unexpectedly renamed file with bad sig"),
        }
    }

    #[tokio::test]
    async fn rename_should_yield_error_if_underlying_path_is_missing() {
        let mut lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");

        let sig = lf.sig();
        match lf.rename(sig, "something_else").await {
            Err(LocalFileRenameError::IoError(_)) => {
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

        let mut lf = LocalFile::new(
            File::from_std(f.as_file().try_clone().unwrap()),
            path,
        );

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
        lf.rename(sig, "renamed_file")
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
        assert_ne!(lf.sig(), sig);
    }

    #[tokio::test]
    async fn remove_should_yield_error_if_provided_sig_is_different() {
        let lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");

        let sig = lf.sig();
        match lf.remove(sig + 1).await {
            Err(LocalFileRemoveError::SigMismatch(lf)) => {
                assert_eq!(lf.sig(), sig, "Signature changed after error");
            }
            Err(x) => panic!("Unexpected error: {}", x),
            Ok(_) => panic!("Unexpectedly removed file with bad sig"),
        }
    }

    #[tokio::test]
    async fn remove_should_yield_error_if_underlying_path_is_missing() {
        let lf =
            LocalFile::new(File::from_std(tempfile::tempfile().unwrap()), "");

        let sig = lf.sig();
        match lf.remove(sig).await {
            Err(LocalFileRemoveError::IoError(_, lf)) => {
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

        let lf = LocalFile::new(
            File::from_std(f.as_file().try_clone().unwrap()),
            path,
        );

        let sig = lf.sig();

        // Do reomve and verify that the file at path is gone
        assert!(fs::read(path).await.is_ok(), "File already missing at path");
        lf.remove(sig).await.unwrap();
        assert!(fs::read(path).await.is_err(), "File still exists at path");
    }

    #[test]
    fn apply_path_changed_should_update_path_if_contains_local_file_path() {
        let from_dir_path = Path::new("/from/path");
        let to_dir_path = Path::new("/to/path");

        let file_path = Path::new("file");
        let from_file_path = from_dir_path.join(file_path);
        let mut lf = LocalFile::new(
            File::from_std(tempfile::tempfile().unwrap()),
            from_file_path,
        );

        assert_eq!(
            lf.path(),
            from_dir_path.join(file_path),
            "LocalFile path not set at proper initial location"
        );

        lf.apply_path_changed(from_dir_path, to_dir_path);

        assert_eq!(
            lf.path(),
            to_dir_path.join(file_path),
            "LocalFile path not updated to new location"
        );
    }

    #[test]
    fn apply_path_changed_should_not_update_path_if_not_contains_local_file_path(
    ) {
        let from_dir_path = Path::new("/from/path");
        let to_dir_path = Path::new("/to/path");

        let file_path = Path::new("file");
        let from_file_path = from_dir_path.join(file_path);
        let mut lf = LocalFile::new(
            File::from_std(tempfile::tempfile().unwrap()),
            from_file_path,
        );

        assert_eq!(
            lf.path(),
            from_dir_path.join(file_path),
            "LocalFile path not set at proper initial location"
        );

        lf.apply_path_changed("/some/other/path", to_dir_path);

        assert_eq!(
            lf.path(),
            from_dir_path.join(file_path),
            "LocalFile path unexpectedly updated to new location"
        );
    }

    #[test]
    fn apply_path_changed_should_update_path_if_is_local_file_path() {
        let from_dir_path = Path::new("/from/path");
        let to_dir_path = Path::new("/to/path");

        let file_path = Path::new("file");
        let from_file_path = from_dir_path.join(file_path);
        let to_file_path = to_dir_path.join(file_path);

        let mut lf = LocalFile::new(
            File::from_std(tempfile::tempfile().unwrap()),
            from_file_path.as_path(),
        );

        assert_eq!(
            lf.path(),
            from_file_path,
            "LocalFile path not set at proper initial location"
        );

        lf.apply_path_changed(from_file_path, to_file_path.as_path());

        assert_eq!(
            lf.path(),
            to_file_path,
            "LocalFile path not updated to new location"
        );
    }
}
