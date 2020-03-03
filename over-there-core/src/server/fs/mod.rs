pub mod dir;
pub mod file;

use dir::LocalDirRenameError;
use file::LocalFile;
use std::collections::{HashMap, HashSet};
use std::io;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Debug)]
struct LocalFilePermissions {
    write: bool,
    read: bool,
}

#[derive(Debug)]
pub struct FileSystemManager {
    files: HashMap<u32, (LocalFile, LocalFilePermissions)>,
    conns: HashMap<SocketAddr, HashSet<u32>>,
}

impl Default for FileSystemManager {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
            conns: HashMap::new(),
        }
    }
}

impl FileSystemManager {
    /// Attempts to rename an entire directory, checking through all open
    /// files to see if their paths also need to be renamed.
    ///
    /// If any file is open, its sig will need to be included in the auth
    /// section to ensure that it can be moved; otherwise, an error will
    /// be returned
    pub async fn rename_dir(
        &mut self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> Result<(), LocalDirRenameError> {
        dir::rename(from, to).await?;

        // TODO: Perform more optimal renames by filtering down open files
        //       using a path tree?
        for f in self.files.values_mut() {
            f.0.apply_path_changed(from, to);
        }

        Ok(())
    }

    /// Opens a file, creating it if `create` true, using `write` and `read`
    /// for permissions.
    ///
    /// If the file is already open, will return the existing instance. If
    /// permissions differ where the returned file does not have read/write
    /// access and the request asks for it, the current instance of the file
    /// will be closed and a new instance with the same id will be opened with
    /// the new permissions where existing and requested permissions align.
    pub async fn open_file(
        &mut self,
        addr: SocketAddr,
        path: impl AsRef<Path>,
        create: bool,
        write: bool,
        read: bool,
    ) -> io::Result<(u32, u32)> {
        let mut new_permissions = LocalFilePermissions { read, write };
        let mut maybe_id_and_sig = None;

        // TODO: Perform more optimal lookup by filtering down open files
        //       using a path tree?
        let search = self
            .files
            .values_mut()
            .find(|f| f.0.path() == path.as_ref());

        // If we found a match, check the permissions to see if we can return
        // it or if we need to open a new copy with the proper merged
        // permissions
        if let Some((file, permissions)) = search {
            let id = file.id();
            let sig = file.sig();

            // We already have read permission or are not asking for it and
            // we already have write permission or are not asking for it
            if (permissions.read || !read) && (permissions.write || !write) {
                self.conns.entry(addr).or_default().insert(id);
                return Ok((id, sig));
            } else {
                // Otherwise, we now need to open a new file pointer with the
                // proper permissions to support both cases and, if successful,
                // close the existing file
                new_permissions.read = permissions.read || read;
                new_permissions.write = permissions.write || write;
                maybe_id_and_sig = Some((id, sig));
            }
        }

        // Open the file with the specified path
        let mut new_file = LocalFile::open(
            path,
            create,
            new_permissions.write,
            new_permissions.read,
        )
        .await?;

        // If we already had a file open with this path, we want to assign
        // the previously-used id and sig
        if let Some((id, sig)) = maybe_id_and_sig {
            new_file.id = id;
            new_file.sig = sig;
        }

        // Insert the file & permissions, overwriting the
        // existing file/permissions
        let id = new_file.id();
        let sig = new_file.sig();
        self.files.insert(id, (new_file, new_permissions));
        self.conns.entry(addr).or_default().insert(id);

        Ok((id, sig))
    }

    /// Closes an open file by id, although the file will not be fully closed
    /// unless all individual connections close the file (or all connections
    /// time out with the open file)
    pub async fn close_file(
        &mut self,
        addr: SocketAddr,
        id: u32,
    ) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }

    /// Looks up an open file by its associated `id`
    pub fn get_mut(&mut self, id: &u32) -> Option<&mut LocalFile> {
        self.files.get_mut(id).map(|f| f.0).as_mut()
    }
}
