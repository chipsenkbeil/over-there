pub mod dir;
pub mod file;

use dir::LocalDirRenameError;
use file::LocalFile;
use std::collections::HashMap;
use std::io;
use std::path::Path;

#[derive(Debug)]
struct LocalFilePermissions {
    write: bool,
    read: bool,
}

#[derive(Debug)]
pub struct FileSystemManager {
    files: HashMap<u32, (LocalFile, LocalFilePermissions)>,
}

impl Default for FileSystemManager {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
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
        dir::rename(from.as_ref(), to.as_ref()).await?;

        // TODO: Perform more optimal renames by filtering down open files
        //       using a path tree?
        for f in self.files.values_mut() {
            f.0.apply_path_changed(from.as_ref(), to.as_ref());
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

        Ok((id, sig))
    }

    /// Closes an open file by `id`, returning whether or not there was a file
    /// to close with the specified `id`
    pub async fn close_file(&mut self, id: u32) -> bool {
        self.files.remove(&id).is_some()
    }

    /// Adds the already-created `local_file`, specifying what permissions
    /// it current has for `write` and `read`.
    ///
    /// NOTE: These permissions MUST match what was specified when opening
    ///       the local file.
    ///
    /// TODO: This is a method only used for testing and should be removed
    ///       when we clean up this interface.
    pub(crate) fn add_existing_file(
        &mut self,
        local_file: LocalFile,
        write: bool,
        read: bool,
    ) -> (u32, u32) {
        let id = local_file.id();
        let sig = local_file.sig();

        let permissions = LocalFilePermissions { read, write };
        self.files.insert(id, (local_file, permissions));

        (id, sig)
    }

    /// Looks up an open file by its associated `id`
    pub fn get_mut(&mut self, id: &u32) -> Option<&mut LocalFile> {
        match self.files.get_mut(id) {
            Some((file, _)) => Some(file),
            None => None,
        }
    }

    /// Looks up an open file by its associated `id`
    pub fn get(&self, id: &u32) -> Option<&LocalFile> {
        match self.files.get(id) {
            Some((file, _)) => Some(file),
            None => None,
        }
    }
}
