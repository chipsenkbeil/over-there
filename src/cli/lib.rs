mod builder;
pub mod format;
mod opts;

use format::FormatOption;
use log::info;
use opts::{
    client::{self, ClientCommand},
    schema::{SchemaSubcommand, SchemaType},
    server::ServerCommand,
    Command,
};
use over_there_core::{
    ConnectedClient, Content, RemoteProc, Reply, SchemaInfo,
};
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub use opts::Opts;

/// Primary entrypoint to run the executable based on input options
pub async fn run(opts: Opts) -> Result<(), Box<dyn Error>> {
    match opts.command {
        Command::Server(s) => run_server(s).await?,
        Command::Client(c) => match (c.output_format, run_client(c).await) {
            (FormatOption::Human, Err(x)) => return Err(x),
            (f, Err(x)) => format::format_content_println(
                f,
                Content::from(Reply::from(x)),
                |_| Err("Cannot write human-readable stderr to stdout".into()),
            )?,
            _ => (),
        },
        Command::Schema(s) => run_schema(s.command).await?,
    };

    Ok(())
}

fn validate_opts(opts: &opts::CommonOpts) -> io::Result<()> {
    if opts.encryption != opts::types::Encryption::None
        && opts.encryption_key.is_none()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Wanted {:?} encryption, but did not provide a key!",
                opts.encryption
            ),
        ));
    }

    if opts.authentication != opts::types::Authentication::None
        && opts.authentication_key.is_none()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Wanted {:?} authentication, but did not provide a key!",
                opts.authentication
            ),
        ));
    }

    Ok(())
}

async fn write_stdout(text: String, path: Option<&PathBuf>) -> io::Result<()> {
    match path {
        Some(p) => {
            use tokio::io::AsyncWriteExt;
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .await?
                .write_all(text.as_bytes())
                .await
        }
        None => {
            print!("{}", text);
            Ok(())
        }
    }
}

async fn write_stderr(text: String, path: Option<&PathBuf>) -> io::Result<()> {
    match path {
        Some(p) => {
            use tokio::io::AsyncWriteExt;
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .await?
                .write_all(text.as_bytes())
                .await
        }
        None => {
            eprint!("{}", text);
            Ok(())
        }
    }
}

async fn run_server(cmd: ServerCommand) -> Result<(), Box<dyn Error>> {
    info!("Launching server: {:?}", cmd);

    validate_opts(&cmd.opts)?;

    let server = builder::start_server(&cmd).await?;

    // Let server run to completion
    server.wait().await?;

    Ok(())
}

/// Enables using args for both human and non-human paths without cloning
macro_rules! format_content_write {
    ($format:expr, $path:expr, $content:expr, $human_expr:expr,) => {
        match $format {
            FormatOption::Human => {
                let result: Result<String, String> = $human_expr;
                match result {
                    Ok(x) => write_stdout(format!("{}", x), $path).await?,
                    Err(x) => write_stderr(format!("{}", x), $path).await?,
                }
                Ok(())
            }
            _ => format::format_content_println($format, $content, |_| {
                Err("Unreachable".into())
            }),
        }
    };
}

async fn run_client(cmd: ClientCommand) -> Result<(), Box<dyn Error>> {
    info!("Launching client: {:?}", cmd);

    validate_opts(&cmd.opts)?;

    let mut client = builder::start_client(&cmd)
        .await
        .expect("Failed to connect with client");

    match &cmd.command {
        client::Subcommand::Version(_) => {
            let x = client.ask_version().await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::Version(x)),
                Ok(x.version),
            )?;
        }
        client::Subcommand::Capabilities(_) => {
            let x = client.ask_capabilities().await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::Capabilities(x)),
                Ok(format!("{:?}", x)),
            )?;
        }
        client::Subcommand::ListRootDir(_) => {
            let x = client.ask_list_root_dir_contents().await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::DirContentsList(x)),
                Ok(x.entries
                    .iter()
                    .map(|e| {
                        format!(
                            "[{}{}{}] {}",
                            if e.is_file { "F" } else { "" },
                            if e.is_dir { "D" } else { "" },
                            if e.is_symlink { "S" } else { "" },
                            e.path,
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n")),
            )?;
        }
        client::Subcommand::ListDir(c) => {
            let x = client.ask_list_dir_contents(c.path.clone()).await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::DirContentsList(x)),
                Ok(x.entries
                    .iter()
                    .map(|e| {
                        format!(
                            "[{}{}{}] {}",
                            if e.is_file { "F" } else { "" },
                            if e.is_dir { "D" } else { "" },
                            if e.is_symlink { "S" } else { "" },
                            e.path,
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n")),
            )?;
        }
        client::Subcommand::CreateDir(c) => {
            let x = client.ask_create_dir(c.path.clone(), c.parents).await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::DirCreated(x)),
                Ok(format!("Created {}", c.path)),
            )?;
        }
        client::Subcommand::MoveDir(c) => {
            let x = client.ask_rename_dir(c.from.clone(), c.to.clone()).await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::DirRenamed(x)),
                Ok(format!("Moved {} to {}", c.from, c.to)),
            )?;
        }
        client::Subcommand::RemoveDir(c) => {
            let x = client.ask_remove_dir(c.path.clone(), c.non_empty).await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::DirRemoved(x)),
                Ok(format!("Removed {}", c.path)),
            )?;
        }
        client::Subcommand::WriteFile(c) => {
            let mut file = client.ask_open_file(c.path.clone()).await?.into();
            let x = client
                .ask_write_file(&mut file, c.contents.as_ref())
                .await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::FileWritten(x)),
                Ok(format!("{:?}", x)),
            )?;
        }
        client::Subcommand::ReadFile(c) => {
            let file = client.ask_open_file(c.path.clone()).await?.into();
            let x = client.ask_read_file(&file).await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::FileContents(x)),
                Ok(String::from_utf8(x.contents)?),
            )?;
        }
        client::Subcommand::MoveFile(c) => {
            let x = client
                .ask_rename_unopened_file(c.from.clone(), c.to.clone())
                .await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::UnopenedFileRenamed(x)),
                Ok(format!("Moved {} to {}", c.from, c.to)),
            )?;
        }
        client::Subcommand::RemoveFile(c) => {
            let x = client.ask_remove_unopened_file(c.path.clone()).await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::UnopenedFileRemoved(x)),
                Ok(format!("Removed {}", c.path)),
            )?;
        }
        client::Subcommand::Exec(c) => {
            let proc = client
                .ask_exec_proc_with_options(
                    c.command.clone(),
                    c.args.clone(),
                    true,
                    true,
                    true,
                    c.current_dir.clone(),
                )
                .await?
                .into();
            process_proc(
                client,
                !c.no_stdin,
                cmd.redirect_stdout,
                cmd.redirect_stderr,
                c.post_exit_duration,
                proc,
                cmd.output_format,
                cmd.exit_print,
            )
            .await?;
        }
        client::Subcommand::ReattachExec(c) => {
            let proc = RemoteProc::shallow(c.id);
            process_proc(
                client,
                !c.no_stdin,
                cmd.redirect_stdout,
                cmd.redirect_stderr,
                c.post_exit_duration,
                proc,
                cmd.output_format,
                cmd.exit_print,
            )
            .await?;
        }
        client::Subcommand::Raw(c) => {
            // If provided some input, attempt to execute it
            if let Some(line) = &c.input {
                execute_raw_and_report(
                    &mut client,
                    &line,
                    c.format,
                    c.format,
                    cmd.redirect_stdout.as_ref(),
                )
                .await?;
            }

            // If marked interactive, continue to read stdin for more lines
            // to execute
            if c.interactive {
                let mut line = String::new();
                while let Ok(n) = std::io::stdin().read_line(&mut line) {
                    if n == 0 {
                        break;
                    }

                    execute_raw_and_report(
                        &mut client,
                        &line,
                        c.format,
                        c.format,
                        cmd.redirect_stdout.as_ref(),
                    )
                    .await?;

                    // NOTE: Must clear line contents before next reading
                    line.clear();
                }
            }
        }
        client::Subcommand::InternalDebug(_) => {
            let x = client.ask_internal_debug().await?;
            format_content_write!(
                cmd.output_format,
                cmd.redirect_stdout.as_ref(),
                Content::from(Reply::InternalDebug(x)),
                Ok(format!("{}", String::from_utf8_lossy(&x.output))),
            )?;
        }
    };

    Ok(())
}

async fn execute_raw(
    client: &mut ConnectedClient,
    input: &str,
    format: FormatOption,
) -> Result<Reply, Box<dyn std::error::Error>> {
    match format::text_to_content(format, input)? {
        Content::Request(x) => Ok(client.ask(x).await?),
        x => Err(format!("Unexpected input: {:?}", x).into()),
    }
}

async fn execute_raw_and_report(
    client: &mut ConnectedClient,
    input: &str,
    input_format: FormatOption,
    output_format: FormatOption,
    redirect_output: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    match execute_raw(client, &input, input_format).await {
        Ok(reply) => Ok(format_content_write!(
            output_format,
            redirect_output,
            Content::from(reply),
            Ok(format!("{:?}", &reply)),
        )?),
        Err(x) => Ok(format_content_write!(
            output_format,
            redirect_output,
            Content::from(Reply::from(x)),
            Ok(format!("{:?}", &x)),
        )?),
    }
}

async fn process_proc(
    mut client: ConnectedClient,
    send_stdin: bool,
    stdout_path: Option<PathBuf>,
    stderr_path: Option<PathBuf>,
    post_exit_duration: Duration,
    proc: RemoteProc,
    format: FormatOption,
    exit_print: bool,
) -> io::Result<()> {
    let stdin = io::stdin();
    let mut exit_instant: Option<Instant> = None;

    // Continue running as long as we haven't exceeded our post-exit duration
    // after the remote process exited
    while exit_instant
        .map(|inst| inst.elapsed() < post_exit_duration)
        .unwrap_or(true)
    {
        if send_stdin {
            use io::BufRead;
            let mut handle = stdin.lock();
            let mut line = String::new();
            match handle.read_line(&mut line) {
                Ok(_) => (),
                Err(x) if x.kind() == io::ErrorKind::WouldBlock => (),
                Err(x) => panic!("Failed to read line of input: {:?}", x),
            }

            if !line.is_empty() {
                client
                    .ask_write_proc_stdin(&proc, &line.into_bytes())
                    .await
                    .expect("Failed to write stdin");
            }
        }

        let stdout_args = client
            .ask_read_proc_stdout(&proc)
            .await
            .expect("Failed to get stdout");
        if !stdout_args.output.is_empty() {
            format_content_write!(
                format,
                stdout_path.as_ref(),
                Content::from(Reply::ProcStdoutContents(stdout_args)),
                Ok(format!("{}", String::from_utf8_lossy(&stdout_args.output))),
            )
            .expect("Failed to format stdout");
        }

        let stderr_args = client
            .ask_read_proc_stderr(&proc)
            .await
            .expect("Failed to get stderr");
        if !stderr_args.output.is_empty() {
            format_content_write!(
                format,
                stderr_path.as_ref(),
                Content::from(Reply::ProcStderrContents(stderr_args)),
                Err(format!(
                    "{}",
                    String::from_utf8_lossy(&stderr_args.output)
                )),
            )
            .expect("Failed to format stderr");
        }

        // Mark ready for exit if proc has exited
        if exit_instant.is_none() {
            let status = client
                .ask_read_proc_status(&proc)
                .await
                .expect("Failed to get proc status");
            if !status.is_alive {
                match format {
                    FormatOption::Human if exit_print => format_content_write!(
                        format,
                        stderr_path.as_ref(),
                        Content::from(Reply::ProcStatus(status)),
                        Err(format!(
                            "Proc {} exited with code {}",
                            status.id,
                            status.exit_code.unwrap_or_default(),
                        )),
                    ),
                    FormatOption::Human => Ok(()),
                    f => format_content_write!(
                        f,
                        stdout_path.as_ref(),
                        Content::from(Reply::ProcStatus(status)),
                        Err("unreachable!".into()),
                    ),
                }
                .expect("Failed to format status");
                exit_instant = Some(Instant::now());
            }
        }
    }

    Ok(())
}

async fn run_schema(cmd: SchemaSubcommand) -> Result<(), Box<dyn Error>> {
    use strum::VariantNames;
    match cmd {
        SchemaSubcommand::List => {
            for v in SchemaType::VARIANTS {
                println!("{}", v);
            }
        }
        SchemaSubcommand::Info(info) => println!(
            "{}",
            match info.schema_type {
                SchemaType::Content => over_there_core::Content::schema(),
                SchemaType::Request => over_there_core::Request::schema(),
                SchemaType::Reply => over_there_core::Reply::schema(),
                SchemaType::HeartbeatRequest => {
                    String::from("{}")
                }
                SchemaType::VersionRequest => {
                    String::from("{}")
                }
                SchemaType::CapabilitiesRequest => {
                    over_there_core::request::CapabilitiesArgs::schema()
                }
                SchemaType::CreateDirRequest => {
                    over_there_core::request::CreateDirArgs::schema()
                }
                SchemaType::RenameDirRequest => {
                    over_there_core::request::RenameDirArgs::schema()
                }
                SchemaType::RemoveDirRequest => {
                    over_there_core::request::RemoveDirArgs::schema()
                }
                SchemaType::ListDirContentsRequest => {
                    over_there_core::request::ListDirContentsArgs::schema()
                }
                SchemaType::OpenFileRequest => {
                    over_there_core::request::OpenFileArgs::schema()
                }
                SchemaType::CloseFileRequest => {
                    over_there_core::request::CloseFileArgs::schema()
                }
                SchemaType::RenameUnopenedFileRequest => {
                    over_there_core::request::RenameUnopenedFileArgs::schema()
                }
                SchemaType::RenameFileRequest => {
                    over_there_core::request::RenameFileArgs::schema()
                }
                SchemaType::RemoveUnopenedFileRequest => {
                    over_there_core::request::RemoveUnopenedFileArgs::schema()
                }
                SchemaType::RemoveFileRequest => {
                    over_there_core::request::RemoveFileArgs::schema()
                }
                SchemaType::ReadFileRequest => {
                    over_there_core::request::ReadFileArgs::schema()
                }
                SchemaType::WriteFileRequest => {
                    over_there_core::request::WriteFileArgs::schema()
                }
                SchemaType::ExecProcRequest => {
                    over_there_core::request::ExecProcArgs::schema()
                }
                SchemaType::WriteProcStdinRequest => {
                    over_there_core::request::WriteProcStdinArgs::schema()
                }
                SchemaType::ReadProcStdoutRequest => {
                    over_there_core::request::ReadProcStdoutArgs::schema()
                }
                SchemaType::ReadProcStderrRequest => {
                    over_there_core::request::ReadProcStderrArgs::schema()
                }
                SchemaType::KillProcRequest => {
                    over_there_core::request::KillProcArgs::schema()
                }
                SchemaType::ReadProcStatusRequest => {
                    over_there_core::request::ReadProcStatusArgs::schema()
                }
                SchemaType::SequenceRequest => {
                    over_there_core::request::SequenceArgs::schema()
                }
                SchemaType::BatchRequest => {
                    over_there_core::request::BatchArgs::schema()
                }
                SchemaType::ForwardRequest => {
                    over_there_core::request::ForwardArgs::schema()
                }
                SchemaType::CustomRequest => {
                    over_there_core::request::CustomArgs::schema()
                }
                SchemaType::InternalDebugRequest => {
                    over_there_core::request::InternalDebugArgs::schema()
                }
                SchemaType::HeartbeatReply => {
                    String::from("{}")
                }
                SchemaType::VersionReply => {
                    over_there_core::reply::VersionArgs::schema()
                }
                SchemaType::CapabilitiesReply => {
                    over_there_core::reply::CapabilitiesArgs::schema()
                }
                SchemaType::CreateDirReply => {
                    over_there_core::reply::DirCreatedArgs::schema()
                }
                SchemaType::RenameDirReply => {
                    over_there_core::reply::DirRenamedArgs::schema()
                }
                SchemaType::RemoveDirReply => {
                    over_there_core::reply::DirRemovedArgs::schema()
                }
                SchemaType::ListDirContentsReply => {
                    over_there_core::reply::DirContentsListArgs::schema()
                }
                SchemaType::OpenFileReply => {
                    over_there_core::reply::FileOpenedArgs::schema()
                }
                SchemaType::CloseFileReply => {
                    over_there_core::reply::FileClosedArgs::schema()
                }
                SchemaType::RenameUnopenedFileReply => {
                    over_there_core::reply::UnopenedFileRenamedArgs::schema()
                }
                SchemaType::RenameFileReply => {
                    over_there_core::reply::FileRenamedArgs::schema()
                }
                SchemaType::RemoveUnopenedFileReply => {
                    over_there_core::reply::UnopenedFileRemovedArgs::schema()
                }
                SchemaType::RemoveFileReply => {
                    over_there_core::reply::FileRemovedArgs::schema()
                }
                SchemaType::ReadFileReply => {
                    over_there_core::reply::FileContentsArgs::schema()
                }
                SchemaType::WriteFileReply => {
                    over_there_core::reply::FileWrittenArgs::schema()
                }
                SchemaType::ExecProcReply => {
                    over_there_core::reply::ProcStartedArgs::schema()
                }
                SchemaType::WriteProcStdinReply => {
                    over_there_core::reply::ProcStdinWrittenArgs::schema()
                }
                SchemaType::ReadProcStdoutReply => {
                    over_there_core::reply::ProcStdoutContentsArgs::schema()
                }
                SchemaType::ReadProcStderrReply => {
                    over_there_core::reply::ProcStderrContentsArgs::schema()
                }
                SchemaType::KillProcReply => {
                    over_there_core::reply::ProcKilledArgs::schema()
                }
                SchemaType::ReadProcStatusReply => {
                    over_there_core::reply::ProcStatusArgs::schema()
                }
                SchemaType::GenericError => {
                    over_there_core::reply::GenericErrorArgs::schema()
                }
                SchemaType::IoError => {
                    over_there_core::reply::IoErrorArgs::schema()
                }
                SchemaType::FileSigChanged => {
                    over_there_core::reply::FileSigChangedArgs::schema()
                }
                SchemaType::SequenceReply => {
                    over_there_core::reply::SequenceArgs::schema()
                }
                SchemaType::BatchReply => {
                    over_there_core::reply::BatchArgs::schema()
                }
                SchemaType::ForwardReply => {
                    over_there_core::reply::ForwardArgs::schema()
                }
                SchemaType::CustomReply => {
                    over_there_core::reply::CustomArgs::schema()
                }
                SchemaType::InternalDebugReply => {
                    over_there_core::reply::InternalDebugArgs::schema()
                }
            }
        ),
    };

    Ok(())
}