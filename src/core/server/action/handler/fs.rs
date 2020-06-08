use crate::core::{
    reply::*,
    request::*,
    server::{
        fs::{LocalDirEntry, LocalFileError, LocalFileHandle},
        state::ServerState,
    },
};
use log::debug;
use std::convert::TryFrom;
use std::io;
use std::sync::Arc;

#[derive(Debug)]
pub enum FileIoError {
    Io(io::Error),
    SigMismatch { id: u32, sig: u32 },
}

impl From<FileIoError> for ReplyError {
    fn from(fie: FileIoError) -> ReplyError {
        match fie {
            FileIoError::Io(x) => ReplyError::Io(x.into()),
            FileIoError::SigMismatch { id, sig } => {
                ReplyError::FileSigChanged(FileSigChangedArgs { id, sig })
            }
        }
    }
}

impl From<FileIoError> for Reply {
    fn from(x: FileIoError) -> Self {
        Self::Error(ReplyError::from(x))
    }
}

pub async fn open_file(
    state: Arc<ServerState>,
    args: &OpenFileArgs,
) -> Result<FileOpenedArgs, io::Error> {
    debug!("handler::open_file: {:?}", args);

    let handle = state
        .fs_manager
        .lock()
        .await
        .open_file(
            &args.path,
            args.create_if_missing,
            args.write_access,
            args.read_access,
        )
        .await?;

    state.touch_file_id(handle.id).await;

    Ok(FileOpenedArgs {
        id: handle.id,
        sig: handle.sig,
        path: args.path.clone(),
        read: args.read_access,
        write: args.write_access,
    })
}

pub async fn close_file(
    state: Arc<ServerState>,
    args: &CloseFileArgs,
) -> Result<FileClosedArgs, io::Error> {
    debug!("handler::close_file: {:?}", args);
    state.touch_file_id(args.id).await;

    let handle = LocalFileHandle {
        id: args.id,
        sig: args.sig,
    };

    let _ = state.fs_manager.lock().await.close_file(handle)?;

    state.remove_file_id(args.id).await;
    Ok(FileClosedArgs { id: args.id })
}

pub async fn rename_unopened_file(
    state: Arc<ServerState>,
    args: &RenameUnopenedFileArgs,
) -> Result<UnopenedFileRenamedArgs, io::Error> {
    debug!("handler::rename_unopened_file: {:?}", args);

    state
        .fs_manager
        .lock()
        .await
        .rename_file(&args.from, &args.to)
        .await?;

    Ok(UnopenedFileRenamedArgs {
        from: args.from.clone(),
        to: args.to.clone(),
    })
}

pub async fn rename_file(
    state: Arc<ServerState>,
    args: &RenameFileArgs,
) -> Result<FileRenamedArgs, FileIoError> {
    debug!("handler::rename_file: {:?}", args);
    state.touch_file_id(args.id).await;

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => match local_file.rename(args.sig, &args.to).await {
            Ok(_) => Ok(FileRenamedArgs {
                id: args.id,
                sig: local_file.sig(),
            }),
            Err(LocalFileError::SigMismatch) => Err(FileIoError::SigMismatch {
                id: args.id,
                sig: local_file.sig(),
            }),
            Err(LocalFileError::IoError(x)) => Err(FileIoError::Io(x)),
        },
        None => Err(FileIoError::Io(
            IoErrorArgs::invalid_file_id(args.id).into(),
        )),
    }
}

pub async fn remove_unopened_file(
    state: Arc<ServerState>,
    args: &RemoveUnopenedFileArgs,
) -> Result<UnopenedFileRemovedArgs, io::Error> {
    debug!("handler::remove_unopened_file: {:?}", args);

    state
        .fs_manager
        .lock()
        .await
        .remove_file(&args.path)
        .await?;

    Ok(UnopenedFileRemovedArgs {
        path: args.path.clone(),
    })
}

pub async fn remove_file(
    state: Arc<ServerState>,
    args: &RemoveFileArgs,
) -> Result<FileRemovedArgs, FileIoError> {
    debug!("handler::remove_file: {:?}", args);
    state.touch_file_id(args.id).await;

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => match local_file.remove(args.sig).await {
            Ok(_) => {
                state.remove_file_id(args.id).await;
                Ok(FileRemovedArgs {
                    id: args.id,
                    sig: local_file.sig(),
                })
            }
            Err(LocalFileError::SigMismatch) => Err(FileIoError::SigMismatch {
                id: args.id,
                sig: local_file.sig(),
            }),
            Err(LocalFileError::IoError(x)) => Err(FileIoError::Io(x)),
        },
        None => Err(FileIoError::Io(
            IoErrorArgs::invalid_file_id(args.id).into(),
        )),
    }
}

pub async fn read_file(
    state: Arc<ServerState>,
    args: &ReadFileArgs,
) -> Result<FileContentsArgs, FileIoError> {
    debug!("handler::read_file: {:?}", args);
    state.touch_file_id(args.id).await;

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => match local_file.read_all(args.sig).await {
            Ok(contents) => Ok(FileContentsArgs {
                id: args.id,
                contents,
            }),
            Err(LocalFileError::SigMismatch) => Err(FileIoError::SigMismatch {
                id: args.id,
                sig: local_file.sig(),
            }),
            Err(LocalFileError::IoError(x)) => Err(FileIoError::Io(x)),
        },
        None => Err(FileIoError::Io(
            IoErrorArgs::invalid_file_id(args.id).into(),
        )),
    }
}

pub async fn write_file(
    state: Arc<ServerState>,
    args: &WriteFileArgs,
) -> Result<FileWrittenArgs, FileIoError> {
    debug!("handler::write_file: {:?}", args);
    state.touch_file_id(args.id).await;

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => {
            match local_file.write_all(args.sig, &args.contents).await {
                Ok(_) => Ok(FileWrittenArgs {
                    id: args.id,
                    sig: local_file.sig(),
                }),
                Err(LocalFileError::SigMismatch) => {
                    Err(FileIoError::SigMismatch {
                        id: args.id,
                        sig: local_file.sig(),
                    })
                }
                Err(LocalFileError::IoError(x)) => Err(FileIoError::Io(x)),
            }
        }
        None => Err(FileIoError::Io(
            IoErrorArgs::invalid_file_id(args.id).into(),
        )),
    }
}

pub async fn create_dir(
    state: Arc<ServerState>,
    args: &CreateDirArgs,
) -> Result<DirCreatedArgs, io::Error> {
    debug!("handler::create_dir: {:?}", args);

    state
        .fs_manager
        .lock()
        .await
        .create_dir(&args.path, args.include_components)
        .await?;

    Ok(DirCreatedArgs {
        path: args.path.clone(),
    })
}

pub async fn rename_dir(
    state: Arc<ServerState>,
    args: &RenameDirArgs,
) -> Result<DirRenamedArgs, io::Error> {
    debug!("handler::rename_dir: {:?}", args);

    state
        .fs_manager
        .lock()
        .await
        .rename_dir(&args.from, &args.to)
        .await?;

    Ok(DirRenamedArgs {
        from: args.from.clone(),
        to: args.to.clone(),
    })
}

pub async fn remove_dir(
    state: Arc<ServerState>,
    args: &RemoveDirArgs,
) -> Result<DirRemovedArgs, io::Error> {
    debug!("handler::remove_dir: {:?}", args);

    state
        .fs_manager
        .lock()
        .await
        .remove_dir(&args.path, args.non_empty)
        .await?;

    Ok(DirRemovedArgs {
        path: args.path.clone(),
    })
}

pub async fn list_dir_contents(
    state: Arc<ServerState>,
    args: &ListDirContentsArgs,
) -> Result<DirContentsListArgs, io::Error> {
    debug!("handler::list_dir_contents: {:?}", args);

    let local_entries = state
        .fs_manager
        .lock()
        .await
        .dir_entries(&args.path)
        .await?;

    let entries = local_entries
        .into_iter()
        .map(DirEntry::try_from)
        .collect::<io::Result<Vec<DirEntry>>>()?;

    Ok(DirContentsListArgs {
        path: args.path.clone(),
        entries,
    })
}

impl TryFrom<LocalDirEntry> for DirEntry {
    type Error = io::Error;

    fn try_from(local_dir_entry: LocalDirEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            path: local_dir_entry
                .path
                .into_os_string()
                .into_string()
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "OS String does not contain valid unicode",
                    )
                })?,
            is_file: local_dir_entry.is_file,
            is_dir: local_dir_entry.is_dir,
            is_symlink: local_dir_entry.is_symlink,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use tokio::fs;

    #[tokio::test]
    async fn open_file_should_return_success_if_create_flag_set_and_opening_new_file(
    ) {
        let state = Arc::new(ServerState::default());
        let tmp_path = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        let args = open_file(
            Arc::clone(&state),
            &OpenFileArgs {
                path: tmp_path.clone(),
                create_if_missing: true,
                write_access: true,
                read_access: true,
            },
        )
        .await
        .unwrap();

        let x = state.fs_manager.lock().await;
        let local_file = x.get(args.id).unwrap();
        assert_eq!(args.sig, local_file.sig());
        assert_eq!(args.path, tmp_path);
        assert!(args.write);
        assert!(args.read);
    }

    #[tokio::test]
    async fn open_file_should_return_success_opening_existing_file() {
        let state = Arc::new(ServerState::default());

        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        let tmp_file_path = tmp_file.path().to_string_lossy().to_string();

        let args = open_file(
            Arc::clone(&state),
            &OpenFileArgs {
                path: tmp_file_path.clone(),
                create_if_missing: false,
                write_access: true,
                read_access: true,
            },
        )
        .await
        .unwrap();

        let x = state.fs_manager.lock().await;
        let local_file = x.get(args.id).unwrap();
        assert_eq!(args.sig, local_file.sig());
        assert_eq!(args.path, tmp_file_path);
        assert!(args.write);
        assert!(args.read);
    }

    #[tokio::test]
    async fn open_file_should_return_error_if_file_missing_and_create_flag_not_set(
    ) {
        let state = Arc::new(ServerState::default());

        let tmp_path = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        let err = open_file(
            Arc::clone(&state),
            &OpenFileArgs {
                path: tmp_path,
                create_if_missing: false,
                write_access: true,
                read_access: true,
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn close_file_should_return_error_if_file_not_open() {
        let state = Arc::new(ServerState::default());

        let tmp_path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(tmp_path, false, false, true)
            .await
            .expect("Failed to open file");

        let id = handle.id + 1;
        let sig = handle.sig;

        let err = close_file(Arc::clone(&state), &CloseFileArgs { id, sig })
            .await
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn close_file_should_return_error_if_signature_different() {
        let state = Arc::new(ServerState::default());

        let tmp_path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(tmp_path, false, false, true)
            .await
            .expect("Failed to open file");

        let id = handle.id;
        let sig = handle.sig + 1;

        let err = close_file(Arc::clone(&state), &CloseFileArgs { id, sig })
            .await
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn close_file_should_return_confirmation_if_successful() {
        let state = Arc::new(ServerState::default());

        let tmp_path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(tmp_path, false, false, true)
            .await
            .expect("Failed to open file");

        let id = handle.id;
        let sig = handle.sig;

        let args = close_file(Arc::clone(&state), &CloseFileArgs { id, sig })
            .await
            .unwrap();

        assert_eq!(args.id, id);
    }

    #[tokio::test]
    async fn rename_unopened_file_should_return_error_if_file_open() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        let err = rename_unopened_file(
            Arc::clone(&state),
            &RenameUnopenedFileArgs {
                from: file.as_ref().to_string_lossy().to_string(),
                to: file.as_ref().to_string_lossy().to_string(),
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn rename_unopened_file_should_return_confirmation_if_file_renamed() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        let from_path_str = file.as_ref().to_string_lossy().to_string();
        let to_path_str = format!("{}.renamed", from_path_str);

        let args = rename_unopened_file(
            Arc::clone(&state),
            &RenameUnopenedFileArgs {
                from: from_path_str.clone(),
                to: to_path_str.clone(),
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(from_path_str.clone()).await.is_err(),
            "File not renamed"
        );
        assert!(
            fs::metadata(to_path_str.clone()).await.is_ok(),
            "File renamed incorrectly"
        );

        assert_eq!(args.from, from_path_str);
        assert_eq!(args.to, to_path_str);
    }

    #[tokio::test]
    async fn rename_file_should_return_error_if_file_not_open() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");
        let new_path_str = String::from("new-file-name");

        let err = rename_file(
            Arc::clone(&state),
            &RenameFileArgs {
                id: handle.id + 1,
                sig: handle.sig,
                to: new_path_str.clone(),
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File missing when rename failed"
        );

        assert!(
            fs::metadata(&new_path_str).await.is_err(),
            "Renamed file exists"
        );

        match err {
            FileIoError::Io(x) => {
                assert_eq!(x.kind(), io::ErrorKind::InvalidInput)
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_file_should_return_error_if_signature_different() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");
        let new_path_str = String::from("new-file-name");

        let err = rename_file(
            Arc::clone(&state),
            &RenameFileArgs {
                id: handle.id,
                sig: handle.sig + 1,
                to: new_path_str.clone(),
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File missing when rename failed"
        );

        assert!(
            fs::metadata(&new_path_str).await.is_err(),
            "Renamed file exists"
        );

        match err {
            FileIoError::SigMismatch { id, sig } => {
                assert_eq!(id, handle.id);
                assert_eq!(sig, handle.sig);
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_file_should_return_confirmation_if_file_renamed() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");
        let new_path_str =
            format!("{}.2", file.as_ref().to_string_lossy().to_string());

        let args = rename_file(
            Arc::clone(&state),
            &RenameFileArgs {
                id: handle.id,
                sig: handle.sig,
                to: new_path_str.clone(),
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_err(),
            "Renamed file still exists at original location"
        );

        assert!(
            fs::metadata(&new_path_str).await.is_ok(),
            "Renamed file missing"
        );

        assert_eq!(handle.id, args.id, "Wrong id returned");
        assert_ne!(handle.sig, args.sig, "Signature returned is not different");
    }

    #[tokio::test]
    async fn remove_unopened_file_should_return_error_if_file_open() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        let err = remove_unopened_file(
            Arc::clone(&state),
            &RemoveUnopenedFileArgs {
                path: file.as_ref().to_string_lossy().to_string(),
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File unexpectedly removed"
        );

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn remove_unopened_file_should_return_confirmation_if_file_removed() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();

        let args = remove_unopened_file(
            Arc::clone(&state),
            &RemoveUnopenedFileArgs {
                path: file.as_ref().to_string_lossy().to_string(),
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_err(),
            "File still exists"
        );

        assert_eq!(args.path, file.as_ref().to_string_lossy().to_string());
    }

    #[tokio::test]
    async fn remove_file_should_return_error_if_file_not_open() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        let err = remove_file(
            Arc::clone(&state),
            &RemoveFileArgs {
                id: handle.id + 1,
                sig: handle.sig,
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File unexpectedly missing after failed remove"
        );

        match err {
            FileIoError::Io(x) => {
                assert_eq!(x.kind(), io::ErrorKind::InvalidInput)
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_file_should_return_error_if_signature_different() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        let err = remove_file(
            Arc::clone(&state),
            &RemoveFileArgs {
                id: handle.id,
                sig: handle.sig + 1,
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File unexpectedly missing after failed remove"
        );

        match err {
            FileIoError::SigMismatch { id, sig } => {
                assert_eq!(id, handle.id);
                assert_eq!(sig, handle.sig);
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_file_should_return_confirmation_if_file_removed() {
        let state = Arc::new(ServerState::default());

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        let args = remove_file(
            Arc::clone(&state),
            &RemoveFileArgs {
                id: handle.id,
                sig: handle.sig,
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_err(),
            "File still exists"
        );

        assert_eq!(handle.id, args.id, "Wrong id returned");
        assert_ne!(handle.sig, args.sig, "Signature returned is not different");
    }

    #[tokio::test]
    async fn read_file_should_return_contents_if_read_successful() {
        let state = Arc::new(ServerState::default());
        let file_contents = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let mut file = tempfile::NamedTempFile::new().unwrap();

        use std::io::Write;
        file.write_all(&file_contents).unwrap();
        file.flush().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), true, true, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        let args = read_file(Arc::clone(&state), &ReadFileArgs { id, sig })
            .await
            .unwrap();

        assert_eq!(args.id, id, "Wrong id returned");
        assert_eq!(args.contents, file_contents);
    }

    #[tokio::test]
    async fn read_file_should_return_error_if_file_not_open() {
        let err = read_file(
            Arc::new(ServerState::default()),
            &ReadFileArgs { id: 0, sig: 0 },
        )
        .await
        .unwrap_err();

        match err {
            FileIoError::Io(x) => {
                assert_eq!(x.kind(), io::ErrorKind::InvalidInput);
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn read_file_should_return_error_if_not_readable() {
        let state = Arc::new(ServerState::default());

        let tmp_file = tempfile::NamedTempFile::new().unwrap();

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .read(false)
            .write(true)
            .create(true)
            .open(tmp_file.path())
            .unwrap();
        file.write_all(&vec![1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap();
        file.flush().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(tmp_file.as_ref(), true, true, false)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        let err = read_file(Arc::clone(&state), &ReadFileArgs { id, sig })
            .await
            .unwrap_err();

        match err {
            FileIoError::Io(x) => {
                assert!(x.raw_os_error().is_some());
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn read_file_should_return_error_if_file_sig_has_changed() {
        let state = Arc::new(ServerState::default());
        let file = tempfile::NamedTempFile::new().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), true, true, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        let err =
            read_file(Arc::clone(&state), &ReadFileArgs { id, sig: sig + 1 })
                .await
                .unwrap_err();

        match err {
            FileIoError::SigMismatch {
                id: cur_id,
                sig: cur_sig,
            } => {
                assert_eq!(cur_id, id);
                assert_eq!(cur_sig, sig);
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn write_file_should_return_success_if_write_successful() {
        let state = Arc::new(ServerState::default());
        let contents = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let mut file = tempfile::NamedTempFile::new().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), true, true, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        let args = write_file(
            Arc::clone(&state),
            &WriteFileArgs {
                id,
                sig,
                contents: contents.clone(),
            },
        )
        .await
        .unwrap();

        assert_eq!(args.id, id, "Wrong id returned");
        assert_ne!(args.sig, sig);

        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0)).unwrap();

        use std::io::Read;
        let mut file_contents = Vec::new();
        file.read_to_end(&mut file_contents).unwrap();

        assert_eq!(
            contents, file_contents,
            "File does not match written content"
        );
    }

    #[tokio::test]
    async fn write_file_should_return_error_if_not_writeable() {
        let state = Arc::new(ServerState::default());
        let contents = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let file = tempfile::NamedTempFile::new().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        let err = write_file(
            Arc::clone(&state),
            &WriteFileArgs { id, sig, contents },
        )
        .await
        .unwrap_err();

        match err {
            FileIoError::Io(x) => {
                // Should be an OS-related error
                assert!(x.raw_os_error().is_some());
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn write_file_should_return_error_if_file_sig_has_changed() {
        let state = Arc::new(ServerState::default());
        let contents = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let file = tempfile::NamedTempFile::new().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), true, true, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        let err = write_file(
            Arc::clone(&state),
            &WriteFileArgs {
                id,
                sig: sig + 1,
                contents: contents.clone(),
            },
        )
        .await
        .unwrap_err();

        match err {
            FileIoError::SigMismatch {
                id: cur_id,
                sig: cur_sig,
            } => {
                assert_eq!(cur_id, id, "Wrong id returned");
                assert_eq!(cur_sig, sig);
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[tokio::test]
    async fn create_dir_should_return_error_if_part_of_path_missing_and_flag_not_set(
    ) {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::default());

        let dir_path = root_path.as_ref().join("test").join("dir");

        let err = create_dir(
            Arc::clone(&state),
            &CreateDirArgs {
                path: dir_path.as_path().to_string_lossy().to_string(),
                include_components: false,
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(&dir_path).await.is_err(),
            "Dir was unexpectedly created"
        );

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn create_dir_should_return_confirmation_if_single_level_directory_created(
    ) {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::default());

        let dir_path = root_path.as_ref().join("test");

        let args = create_dir(
            Arc::clone(&state),
            &CreateDirArgs {
                path: dir_path.as_path().to_string_lossy().to_string(),
                include_components: false,
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(&dir_path).await.is_ok(),
            "Dir was unexpectedly not created"
        );

        assert_eq!(
            args.path,
            dir_path.to_string_lossy().to_string(),
            "Wrong path returned"
        );
    }

    #[tokio::test]
    async fn create_dir_should_return_confirmation_if_multi_level_directory_created(
    ) {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::default());

        let dir_path = root_path.as_ref().join("test").join("dir");

        let args = create_dir(
            Arc::clone(&state),
            &CreateDirArgs {
                path: dir_path.as_path().to_string_lossy().to_string(),
                include_components: true,
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(&dir_path).await.is_ok(),
            "Dir was unexpectedly not created"
        );
        assert_eq!(
            args.path,
            dir_path.to_string_lossy().to_string(),
            "Wrong path returned"
        );
    }

    #[tokio::test]
    async fn rename_dir_should_return_error_if_a_file_open_in_directory() {
        let state = Arc::new(ServerState::default());

        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap().into_path();
        fs::remove_dir(to_dir.as_path())
            .await
            .expect("Failed to clean up temp dir");

        let file_path = from_dir.as_ref().join("test-file");
        let _ = state
            .fs_manager
            .lock()
            .await
            .open_file(file_path.as_path(), true, true, true)
            .await
            .expect("Failed to open file");

        let err = rename_dir(
            Arc::clone(&state),
            &RenameDirArgs {
                from: from_dir.as_ref().to_string_lossy().to_string(),
                to: to_dir.as_path().to_string_lossy().to_string(),
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(file_path.as_path()).await.is_ok(),
            "Open file unexpectedly renamed to something else"
        );

        assert!(
            fs::metadata(from_dir.as_ref()).await.is_ok(),
            "Dir was unexpectedly renamed"
        );

        assert!(
            fs::metadata(to_dir.as_path()).await.is_err(),
            "Destination of rename unexpectedly exists"
        );

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn rename_dir_should_return_confirmation_if_directory_renamed() {
        let state = Arc::new(ServerState::default());

        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap().into_path();
        fs::remove_dir(to_dir.as_path())
            .await
            .expect("Failed to clean up temp dir");

        let args = rename_dir(
            Arc::clone(&state),
            &RenameDirArgs {
                from: from_dir.as_ref().to_string_lossy().to_string(),
                to: to_dir.as_path().to_string_lossy().to_string(),
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(from_dir.as_ref()).await.is_err(),
            "Dir was unexpectedly not renamed"
        );

        assert!(
            fs::metadata(to_dir.as_path()).await.is_ok(),
            "Destination of rename unexpectedly missing"
        );

        assert_eq!(
            args.from,
            from_dir.as_ref().to_string_lossy().to_string(),
            "Wrong from path returned"
        );
        assert_eq!(
            args.to,
            to_dir.to_string_lossy().to_string(),
            "Wrong to path returned"
        );
    }

    #[tokio::test]
    async fn remove_dir_should_return_error_if_a_file_open_in_directory() {
        let state = Arc::new(ServerState::default());

        let dir = tempfile::tempdir().unwrap();

        let file_path = dir.as_ref().join("test-file");
        let _ = state
            .fs_manager
            .lock()
            .await
            .open_file(file_path.as_path(), true, true, true)
            .await
            .expect("Failed to open file");

        let err = remove_dir(
            Arc::clone(&state),
            &RemoveDirArgs {
                path: dir.as_ref().to_string_lossy().to_string(),
                non_empty: true,
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(file_path.as_path()).await.is_ok(),
            "Open file unexpectedly removed"
        );

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn remove_dir_should_return_confirmation_if_empty_directory_removed()
    {
        let state = Arc::new(ServerState::default());

        let dir = tempfile::tempdir().unwrap();

        let args = remove_dir(
            Arc::clone(&state),
            &RemoveDirArgs {
                path: dir.as_ref().to_string_lossy().to_string(),
                non_empty: false,
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(dir.as_ref()).await.is_err(),
            "Dir still exists"
        );

        assert_eq!(
            args.path,
            dir.as_ref().to_string_lossy().to_string(),
            "Wrong path returned"
        );
    }

    #[tokio::test]
    async fn remove_dir_should_return_error_if_nonempty_directory_removed_with_flag_not_set(
    ) {
        let state = Arc::new(ServerState::default());

        let dir = tempfile::tempdir().unwrap();
        let file = tempfile::NamedTempFile::new_in(dir.as_ref()).unwrap();

        let err = remove_dir(
            Arc::clone(&state),
            &RemoveDirArgs {
                path: dir.as_ref().to_string_lossy().to_string(),
                non_empty: false,
            },
        )
        .await
        .unwrap_err();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File in dir unexpectedly removed"
        );

        assert!(
            fs::metadata(dir.as_ref()).await.is_ok(),
            "Dir unexpected removed"
        );

        assert_eq!(err.kind(), io::ErrorKind::Other);
    }

    #[tokio::test]
    async fn remove_dir_should_return_confirmation_if_nonempty_directory_removed_with_flag_set(
    ) {
        let state = Arc::new(ServerState::default());

        let dir = tempfile::tempdir().unwrap();
        let file = tempfile::NamedTempFile::new_in(dir.as_ref()).unwrap();

        let args = remove_dir(
            Arc::clone(&state),
            &RemoveDirArgs {
                path: dir.as_ref().to_string_lossy().to_string(),
                non_empty: true,
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_err(),
            "File in dir still exists"
        );

        assert!(
            fs::metadata(dir.as_ref()).await.is_err(),
            "Dir still exists"
        );

        assert_eq!(
            args.path,
            dir.as_ref().to_string_lossy().to_string(),
            "Wrong path returned"
        );
    }

    #[tokio::test]
    async fn list_dir_contents_should_return_entries_if_successful() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_string_lossy().to_string();

        let tmp_file = tempfile::NamedTempFile::new_in(&dir).unwrap();
        let tmp_file_path = fs::canonicalize(tmp_file.path())
            .await
            .unwrap()
            .to_string_lossy()
            .to_string();

        let tmp_dir = tempfile::tempdir_in(&dir).unwrap();
        let tmp_dir_path = fs::canonicalize(tmp_dir.path())
            .await
            .unwrap()
            .to_string_lossy()
            .to_string();

        let args = list_dir_contents(
            Arc::new(ServerState::default()),
            &ListDirContentsArgs {
                path: dir_path.clone(),
            },
        )
        .await
        .unwrap();

        std::fs::remove_dir_all(dir_path).unwrap();

        assert_eq!(args.entries.len(), 2, "Unexpected number of entries");

        assert!(args.entries.contains(&DirEntry {
            path: tmp_file_path,
            is_file: true,
            is_dir: false,
            is_symlink: false
        }));

        assert!(args.entries.contains(&DirEntry {
            path: tmp_dir_path,
            is_file: false,
            is_dir: true,
            is_symlink: false
        }));
    }

    #[tokio::test]
    async fn list_dir_contents_should_return_error_if_path_invalid() {
        let err = list_dir_contents(
            Arc::new(ServerState::default()),
            &ListDirContentsArgs {
                path: String::from(""),
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
