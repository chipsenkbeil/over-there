mod builder;
pub mod format;
mod opts;

use crate::core::{ConnectedClient, Content, RemoteProc, Reply, SchemaInfo};
use format::FormatOption;
use log::info;
use opts::{
    client::{self, ClientCommand},
    schema::{SchemaSubcommand, SchemaType},
    server::ServerCommand,
    Command,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub use opts::Opts;

pub type Metadata = HashMap<String, String>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentAndMetadata {
    content: Content,
    metadata: Metadata,
}

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
                    c.meta_mode,
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
                        c.meta_mode,
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

async fn execute_raw_content(
    client: &mut ConnectedClient,
    input: &str,
    format: FormatOption,
) -> Result<Reply, Box<dyn std::error::Error>> {
    let content: Content = format::convert_text(format, input)?;
    match content {
        Content::Request(x) => Ok(client.ask(x).await?),
        x => Err(format!("Unexpected input: {:?}", x).into()),
    }
}

async fn execute_raw_content_and_metadata(
    client: &mut ConnectedClient,
    input: &str,
    format: FormatOption,
) -> Result<
    (
        Result<Reply, Box<dyn std::error::Error>>,
        HashMap<String, String>,
    ),
    Box<dyn std::error::Error>,
> {
    let content_and_metadata: ContentAndMetadata =
        format::convert_text(format, input)?;
    match content_and_metadata {
        ContentAndMetadata {
            content: Content::Request(x),
            metadata,
        } => Ok((client.ask(x).await.map_err(Box::from), metadata)),
        x => Err(format!("Unexpected input: {:?}", x).into()),
    }
}

async fn execute_raw_and_report(
    client: &mut ConnectedClient,
    input: &str,
    input_format: FormatOption,
    output_format: FormatOption,
    meta_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if meta_mode {
        match execute_raw_content_and_metadata(client, &input, input_format)
            .await
        {
            Ok((result, metadata)) => format::format_println(
                output_format,
                ContentAndMetadata {
                    content: match result {
                        Ok(reply) => Content::from(reply),
                        Err(x) => Content::from(Reply::from(x)),
                    },
                    metadata,
                },
                |_| Err("Unreachable".into()),
            ),
            Err(x) => format::format_println(
                output_format,
                ContentAndMetadata {
                    content: Content::from(Reply::from(x)),
                    metadata: HashMap::new(),
                },
                |_| Err("Unreachable".into()),
            ),
        }
    } else {
        match execute_raw_content(client, &input, input_format).await {
            Ok(reply) => format::format_content_println(
                output_format,
                Content::from(reply),
                |_| Err("Unreachable".into()),
            ),
            Err(x) => format::format_content_println(
                output_format,
                Content::from(Reply::from(x)),
                |_| Err("Unreachable".into()),
            ),
        }
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
                SchemaType::Content => crate::core::Content::schema(),
                SchemaType::Request => crate::core::Request::schema(),
                SchemaType::Reply => crate::core::Reply::schema(),
                SchemaType::HeartbeatRequest => {
                    String::from("{}")
                }
                SchemaType::VersionRequest => {
                    String::from("{}")
                }
                SchemaType::CapabilitiesRequest => {
                    crate::core::request::CapabilitiesArgs::schema()
                }
                SchemaType::CreateDirRequest => {
                    crate::core::request::CreateDirArgs::schema()
                }
                SchemaType::RenameDirRequest => {
                    crate::core::request::RenameDirArgs::schema()
                }
                SchemaType::RemoveDirRequest => {
                    crate::core::request::RemoveDirArgs::schema()
                }
                SchemaType::ListDirContentsRequest => {
                    crate::core::request::ListDirContentsArgs::schema()
                }
                SchemaType::OpenFileRequest => {
                    crate::core::request::OpenFileArgs::schema()
                }
                SchemaType::CloseFileRequest => {
                    crate::core::request::CloseFileArgs::schema()
                }
                SchemaType::RenameUnopenedFileRequest => {
                    crate::core::request::RenameUnopenedFileArgs::schema()
                }
                SchemaType::RenameFileRequest => {
                    crate::core::request::RenameFileArgs::schema()
                }
                SchemaType::RemoveUnopenedFileRequest => {
                    crate::core::request::RemoveUnopenedFileArgs::schema()
                }
                SchemaType::RemoveFileRequest => {
                    crate::core::request::RemoveFileArgs::schema()
                }
                SchemaType::ReadFileRequest => {
                    crate::core::request::ReadFileArgs::schema()
                }
                SchemaType::WriteFileRequest => {
                    crate::core::request::WriteFileArgs::schema()
                }
                SchemaType::ExecProcRequest => {
                    crate::core::request::ExecProcArgs::schema()
                }
                SchemaType::WriteProcStdinRequest => {
                    crate::core::request::WriteProcStdinArgs::schema()
                }
                SchemaType::ReadProcStdoutRequest => {
                    crate::core::request::ReadProcStdoutArgs::schema()
                }
                SchemaType::ReadProcStderrRequest => {
                    crate::core::request::ReadProcStderrArgs::schema()
                }
                SchemaType::KillProcRequest => {
                    crate::core::request::KillProcArgs::schema()
                }
                SchemaType::ReadProcStatusRequest => {
                    crate::core::request::ReadProcStatusArgs::schema()
                }
                SchemaType::SequenceRequest => {
                    crate::core::request::SequenceArgs::schema()
                }
                SchemaType::BatchRequest => {
                    crate::core::request::BatchArgs::schema()
                }
                SchemaType::ForwardRequest => {
                    crate::core::request::ForwardArgs::schema()
                }
                SchemaType::CustomRequest => {
                    crate::core::request::CustomArgs::schema()
                }
                SchemaType::InternalDebugRequest => {
                    crate::core::request::InternalDebugArgs::schema()
                }
                SchemaType::HeartbeatReply => {
                    String::from("{}")
                }
                SchemaType::VersionReply => {
                    crate::core::reply::VersionArgs::schema()
                }
                SchemaType::CapabilitiesReply => {
                    crate::core::reply::CapabilitiesArgs::schema()
                }
                SchemaType::CreateDirReply => {
                    crate::core::reply::DirCreatedArgs::schema()
                }
                SchemaType::RenameDirReply => {
                    crate::core::reply::DirRenamedArgs::schema()
                }
                SchemaType::RemoveDirReply => {
                    crate::core::reply::DirRemovedArgs::schema()
                }
                SchemaType::ListDirContentsReply => {
                    crate::core::reply::DirContentsListArgs::schema()
                }
                SchemaType::OpenFileReply => {
                    crate::core::reply::FileOpenedArgs::schema()
                }
                SchemaType::CloseFileReply => {
                    crate::core::reply::FileClosedArgs::schema()
                }
                SchemaType::RenameUnopenedFileReply => {
                    crate::core::reply::UnopenedFileRenamedArgs::schema()
                }
                SchemaType::RenameFileReply => {
                    crate::core::reply::FileRenamedArgs::schema()
                }
                SchemaType::RemoveUnopenedFileReply => {
                    crate::core::reply::UnopenedFileRemovedArgs::schema()
                }
                SchemaType::RemoveFileReply => {
                    crate::core::reply::FileRemovedArgs::schema()
                }
                SchemaType::ReadFileReply => {
                    crate::core::reply::FileContentsArgs::schema()
                }
                SchemaType::WriteFileReply => {
                    crate::core::reply::FileWrittenArgs::schema()
                }
                SchemaType::ExecProcReply => {
                    crate::core::reply::ProcStartedArgs::schema()
                }
                SchemaType::WriteProcStdinReply => {
                    crate::core::reply::ProcStdinWrittenArgs::schema()
                }
                SchemaType::ReadProcStdoutReply => {
                    crate::core::reply::ProcStdoutContentsArgs::schema()
                }
                SchemaType::ReadProcStderrReply => {
                    crate::core::reply::ProcStderrContentsArgs::schema()
                }
                SchemaType::KillProcReply => {
                    crate::core::reply::ProcKilledArgs::schema()
                }
                SchemaType::ReadProcStatusReply => {
                    crate::core::reply::ProcStatusArgs::schema()
                }
                SchemaType::ErrorReply => {
                    crate::core::reply::ReplyError::schema()
                }
                SchemaType::GenericError => {
                    crate::core::reply::GenericErrorArgs::schema()
                }
                SchemaType::IoError => {
                    crate::core::reply::IoErrorArgs::schema()
                }
                SchemaType::FileSigChanged => {
                    crate::core::reply::FileSigChangedArgs::schema()
                }
                SchemaType::SequenceReply => {
                    crate::core::reply::SequenceArgs::schema()
                }
                SchemaType::BatchReply => {
                    crate::core::reply::BatchArgs::schema()
                }
                SchemaType::ForwardReply => {
                    crate::core::reply::ForwardArgs::schema()
                }
                SchemaType::CustomReply => {
                    crate::core::reply::CustomArgs::schema()
                }
                SchemaType::InternalDebugReply => {
                    crate::core::reply::InternalDebugArgs::schema()
                }
            }
        ),
    };

    Ok(())
}
