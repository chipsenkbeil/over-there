use crate::{
    msg::content::{
        io::{file::*, IoErrorArgs},
        Content,
    },
    server::{action::ActionError, file::LocalFile, state::ServerState},
};
use log::debug;
use rand::RngCore;
use std::io;
use tokio::fs::{self, OpenOptions};

pub async fn do_open_file(
    state: &mut ServerState,
    args: &DoOpenFileArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_open_file: {:?}", args);

    match OpenOptions::new()
        .create(args.create_if_missing)
        .write(args.write_access)
        .read(true)
        .open(&args.path)
        .await
    {
        Ok(file) => {
            let mut r = rand::thread_rng();
            let id = r.next_u32();
            let sig = r.next_u32();

            // Store the opened file so we can operate on it later
            state.files.insert(id, LocalFile { id, sig, file });

            respond(Content::FileOpened(FileOpenedArgs { id, sig }))
        }
        Err(x) => respond(Content::IoError(From::from(x))),
    }
}

pub async fn do_read_file(
    state: &mut ServerState,
    args: &DoReadFileArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_read_file: {:?}", args);

    match state.files.get_mut(&args.id) {
        Some(local_file) => {
            if local_file.sig == args.sig {
                match do_read_file_impl(&mut local_file.file).await {
                    Ok(data) => respond(Content::FileContents(FileContentsArgs { data })),
                    Err(x) => respond(Content::IoError(x)),
                }
            } else {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    sig: local_file.sig,
                }))
            }
        }
        None => respond(Content::IoError(IoErrorArgs::invalid_file_id(args.id))),
    }
}

async fn do_read_file_impl(file: &mut fs::File) -> Result<Vec<u8>, IoErrorArgs> {
    use std::io::SeekFrom;
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();

    file.seek(SeekFrom::Start(0))
        .await
        .map_err(|e| IoErrorArgs::from_error_with_prefix(e, "Seek(0): "))?;

    file.read_to_end(&mut buf)
        .await
        .map_err(|e| IoErrorArgs::from_error_with_prefix(e, "ReadToEnd: "))?;

    Ok(buf)
}

pub async fn do_write_file(
    state: &mut ServerState,
    args: &DoWriteFileArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_write_file: {:?}", args);

    match state.files.get_mut(&args.id) {
        Some(local_file) => {
            if local_file.sig == args.sig {
                match do_write_file_impl(&mut local_file.file, &args.data).await {
                    Ok(_) => {
                        let new_sig = rand::thread_rng().next_u32();
                        local_file.sig = new_sig;

                        respond(Content::FileWritten(FileWrittenArgs { sig: new_sig }))
                    }
                    Err(x) => respond(Content::IoError(x)),
                }
            } else {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    sig: local_file.sig,
                }))
            }
        }
        None => respond(Content::IoError(IoErrorArgs::invalid_file_id(args.id))),
    }
}

async fn do_write_file_impl(file: &mut fs::File, buf: &[u8]) -> Result<(), IoErrorArgs> {
    use std::io::SeekFrom;
    use tokio::io::AsyncWriteExt;

    file.seek(SeekFrom::Start(0))
        .await
        .map_err(|e| IoErrorArgs::from_error_with_prefix(e, "Seek(0): "))?;

    file.set_len(0)
        .await
        .map_err(|e| IoErrorArgs::from_error_with_prefix(e, "SetLen(0): "))?;

    file.write_all(buf)
        .await
        .map_err(|e| IoErrorArgs::from_error_with_prefix(e, "WriteAll: "))
}

pub async fn do_list_dir_contents(
    _state: &mut ServerState,
    args: &DoListDirContentsArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_list_dir_contents: {:?}", args);

    match lookup_entries(&args.path).await {
        Ok(entries) => respond(Content::DirContentsList(From::from(entries))),
        Err(x) => respond(Content::IoError(From::from(x))),
    }
}

async fn lookup_entries(path: &str) -> Result<Vec<DirEntry>, io::Error> {
    let mut entries = Vec::new();
    let mut dir_stream = fs::read_dir(path).await?;
    while let Some(entry) = dir_stream.next_entry().await? {
        let file_type = entry.file_type().await?;
        entries.push(DirEntry {
            path: entry.path().into_os_string().into_string().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "OS String does not contain valid unicode",
                )
            })?,
            is_file: file_type.is_file(),
            is_dir: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn do_open_file_should_send_success_if_create_flag_set_and_opening_new_file() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let tmp_path = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        do_open_file(
            &mut state,
            &DoOpenFileArgs {
                path: tmp_path,
                create_if_missing: true,
                write_access: true,
                read_access: true,
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileOpened(args) => {
                let local_file = state.files.get(&args.id).unwrap();
                assert_eq!(args.sig, local_file.sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_open_file_should_send_success_opening_existing_file() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        let tmp_file_path = tmp_file.path().to_string_lossy().to_string();

        do_open_file(
            &mut state,
            &DoOpenFileArgs {
                path: tmp_file_path,
                create_if_missing: false,
                write_access: true,
                read_access: true,
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileOpened(args) => {
                let local_file = state.files.get(&args.id).unwrap();
                assert_eq!(args.sig, local_file.sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_open_file_should_send_error_if_file_missing_and_create_flag_not_set() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let tmp_path = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        do_open_file(
            &mut state,
            &DoOpenFileArgs {
                path: tmp_path,
                create_if_missing: false,
                write_access: true,
                read_access: true,
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::NotFound)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_contents_if_read_successful() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;
        let file_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let id = 999;
        let sig = 12345;
        let mut file = tempfile::tempfile().unwrap();

        use std::io::Write;
        file.write_all(&file_data).unwrap();
        file.flush().unwrap();

        state.files.insert(
            id,
            LocalFile {
                id,
                sig,
                file: tokio::fs::File::from_std(file),
            },
        );

        do_read_file(&mut state, &DoReadFileArgs { id, sig }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileContents(FileContentsArgs { data }) => {
                assert_eq!(data, file_data);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_error_if_file_not_open() {
        let mut content: Option<Content> = None;

        do_read_file(
            &mut ServerState::default(),
            &DoReadFileArgs { id: 0, sig: 0 },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_error_if_not_readable() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let sig = 12345;
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

        state.files.insert(
            id,
            LocalFile {
                id,
                sig,
                file: tokio::fs::File::from_std(file),
            },
        );

        do_read_file(&mut state, &DoReadFileArgs { id, sig }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { os_code, .. }) => {
                // Should be an OS-related error
                assert!(os_code.is_some());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_error_if_file_sig_has_changed() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let cur_sig = 12345;
        let file = tempfile::tempfile().unwrap();
        state.files.insert(
            id,
            LocalFile {
                id,
                sig: cur_sig,
                file: tokio::fs::File::from_std(file),
            },
        );

        do_read_file(&mut state, &DoReadFileArgs { id, sig: 99999 }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileSigChanged(FileSigChangedArgs { sig }) => {
                assert_eq!(sig, cur_sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_file_should_send_success_if_write_successful() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let id = 999;
        let sig = 12345;
        let mut file = tempfile::tempfile().unwrap();
        state.files.insert(
            id,
            LocalFile {
                id,
                sig,
                file: tokio::fs::File::from_std(file.try_clone().unwrap()),
            },
        );

        do_write_file(
            &mut state,
            &DoWriteFileArgs {
                id,
                sig,
                data: data.clone(),
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileWritten(FileWrittenArgs { sig: new_sig }) => {
                assert_ne!(new_sig, sig);

                use std::io::{Seek, SeekFrom};
                file.seek(SeekFrom::Start(0)).unwrap();

                use std::io::Read;
                let mut file_data = Vec::new();
                file.read_to_end(&mut file_data).unwrap();

                assert_eq!(data, file_data, "File does not match written content");
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_file_should_send_error_if_not_writeable() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;

        let id = 999;
        let sig = 12345;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(tmp_file.path())
            .unwrap();
        state.files.insert(
            id,
            LocalFile {
                id,
                sig,
                file: tokio::fs::File::from_std(file),
            },
        );

        do_write_file(&mut state, &DoWriteFileArgs { id, sig, data }, |c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { os_code, .. }) => {
                // Should be an OS-related error
                assert!(os_code.is_some());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_file_should_send_error_if_file_sig_has_changed() {
        let mut state = ServerState::default();
        let mut content: Option<Content> = None;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let id = 999;
        let cur_sig = 12345;
        let file = tempfile::tempfile().unwrap();
        state.files.insert(
            id,
            LocalFile {
                id,
                sig: cur_sig,
                file: tokio::fs::File::from_std(file),
            },
        );

        do_write_file(
            &mut state,
            &DoWriteFileArgs {
                id,
                sig: 99999,
                data: data.clone(),
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileSigChanged(FileSigChangedArgs { sig }) => {
                assert_eq!(sig, cur_sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_list_dir_contents_should_send_entries_if_successful() {
        let mut content: Option<Content> = None;

        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_string_lossy().to_string();

        let tmp_file = tempfile::NamedTempFile::new_in(&dir).unwrap();
        let tmp_dir = tempfile::tempdir_in(&dir).unwrap();

        do_list_dir_contents(
            &mut ServerState::default(),
            &DoListDirContentsArgs {
                path: dir_path.clone(),
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        std::fs::remove_dir_all(dir_path).unwrap();

        match content.unwrap() {
            Content::DirContentsList(args) => {
                assert_eq!(args.entries.len(), 2, "Unexpected number of entries");

                assert!(args.entries.contains(&DirEntry {
                    path: tmp_file.path().to_string_lossy().to_string(),
                    is_file: true,
                    is_dir: false,
                    is_symlink: false
                }));

                assert!(args.entries.contains(&DirEntry {
                    path: tmp_dir.path().to_string_lossy().to_string(),
                    is_file: false,
                    is_dir: true,
                    is_symlink: false
                }));
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_list_dir_contents_should_send_error_if_path_invalid() {
        let mut content: Option<Content> = None;

        do_list_dir_contents(
            &mut ServerState::default(),
            &DoListDirContentsArgs {
                path: String::from(""),
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::NotFound)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }
}
