use crate::{
    msg::content::{file::*, Content},
    server::{action::ActionError, file::LocalFile, state::ServerState},
};
use log::debug;
use rand::RngCore;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind};

pub fn do_open_file(
    state: &mut ServerState,
    args: &DoOpenFileArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_open_file: {:?}", args);

    match OpenOptions::new()
        .create(args.create_if_missing)
        .write(args.write_access)
        .open(&args.path)
    {
        Ok(file) => {
            let mut r = rand::thread_rng();
            let id = r.next_u32();
            let sig = r.next_u32();

            // Store the opened file so we can operate on it later
            state.files.insert(id, LocalFile { id, sig, file });

            respond(Content::FileOpened(FileOpenedArgs { id, sig }))
        }
        Err(x) => {
            use std::error::Error;
            respond(Content::FileError(FileErrorArgs {
                description: x.description().to_string(),
                error_kind: x.kind(),
            }))
        }
    }
}

pub fn do_read_file(
    state: &mut ServerState,
    args: &DoReadFileArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_read_file: {:?}", args);

    match state.files.get_mut(&args.id) {
        Some(local_file) => {
            if local_file.sig == args.sig {
                match {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    local_file.file.read_to_end(&mut buf).map(|_| buf)
                } {
                    Ok(data) => respond(Content::FileContents(FileContentsArgs { data })),
                    Err(x) => {
                        use std::error::Error;
                        respond(Content::FileError(FileErrorArgs {
                            description: x.description().to_string(),
                            error_kind: x.kind(),
                        }))
                    }
                }
            } else {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    sig: local_file.sig,
                }))
            }
        }
        None => respond(Content::FileError(FileErrorArgs {
            description: format!("No file open with id {}", args.id),
            error_kind: ErrorKind::InvalidInput,
        })),
    }
}

pub fn do_write_file(
    state: &mut ServerState,
    args: &DoWriteFileArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_write_file: {:?}", args);

    match state.files.get_mut(&args.id) {
        Some(local_file) => {
            if local_file.sig == args.sig {
                use std::io::Write;
                match local_file.file.write_all(&args.data) {
                    Ok(_) => {
                        let new_sig = rand::thread_rng().next_u32();
                        local_file.sig = new_sig;

                        respond(Content::FileWritten(FileWrittenArgs { sig: new_sig }))
                    }
                    Err(x) => {
                        use std::error::Error;
                        respond(Content::FileError(FileErrorArgs {
                            description: x.description().to_string(),
                            error_kind: x.kind(),
                        }))
                    }
                }
            } else {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    sig: local_file.sig,
                }))
            }
        }
        None => respond(Content::FileError(FileErrorArgs {
            description: format!("No file open with id {}", args.id),
            error_kind: ErrorKind::InvalidInput,
        })),
    }
}

pub fn do_list_dir_contents(
    _state: &mut ServerState,
    args: &DoListDirContentsArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_list_dir_contents: {:?}", args);

    let lookup_entries = |path| -> Result<Vec<DirEntry>, io::Error> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
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
    };

    match lookup_entries(&args.path) {
        Ok(entries) => respond(Content::DirContentsList(DirContentsListArgs { entries })),
        Err(x) => {
            use std::error::Error;
            respond(Content::FileError(FileErrorArgs {
                description: x.description().to_string(),
                error_kind: x.kind(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_open_file_should_send_success_if_no_io_error_occurs() {
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
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .unwrap();

        match content.unwrap() {
            Content::FileOpened(args) => {
                let local_file = state.files.get(&args.id).unwrap();
                assert_eq!(args.sig, local_file.sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_open_file_should_send_error_if_file_missing_and_create_flag_not_set() {
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
            },
            |c| {
                content = Some(c);
                Ok(())
            },
        )
        .unwrap();

        match content.unwrap() {
            Content::FileError(FileErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::NotFound)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[test]
    fn do_open_file_should_send_error_if_io_error_occurs() {
        unimplemented!();
    }

    #[test]
    fn do_read_file_should_send_contents_if_read_successful() {
        unimplemented!();
    }

    #[test]
    fn do_read_file_should_send_error_if_io_error_occurs() {
        unimplemented!();
    }

    #[test]
    fn do_read_file_should_send_error_if_file_not_open() {
        unimplemented!();
    }

    #[test]
    fn do_read_file_should_send_error_if_file_sig_has_changed() {
        unimplemented!();
    }

    #[test]
    fn do_write_file_should_send_success_if_write_successful() {
        unimplemented!();
    }

    #[test]
    fn do_write_file_should_send_error_if_io_error_occurs() {
        unimplemented!();
    }

    #[test]
    fn do_write_file_should_send_error_if_file_sig_has_changed() {
        unimplemented!();
    }

    #[test]
    fn do_list_dir_contents_should_send_entries_if_successful() {
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
        .unwrap();

        fs::remove_dir_all(dir_path).unwrap();

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

    #[test]
    fn do_list_dir_contents_should_send_error_if_path_invalid() {
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
        .unwrap();

        match content.unwrap() {
            Content::FileError(FileErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::NotFound)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }
}
