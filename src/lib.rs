mod builder;
pub mod format;
mod opts;

use format::FormatOption;
use log::info;
use opts::{
    client::{self, ClientCommand},
    server::ServerCommand,
    Command,
};
use over_there_core::{ConnectedClient, Content, RemoteProc};
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

pub use opts::Opts;

/// Primary entrypoint to run the executable based on input options
pub async fn run(opts: Opts) -> Result<(), Box<dyn Error>> {
    match opts.command {
        Command::Server(s) => run_server(s).await?,
        Command::Client(c) => match (c.format, run_client(c).await) {
            (FormatOption::Human, Err(x)) => return Err(x),
            (f, Err(x)) => format::format_content_println(
                f,
                Content::Error(x.into()),
                |_| Err("Cannot write human-readable stderr to stdout".into()),
            )?,
            _ => (),
        },
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

async fn run_server(cmd: ServerCommand) -> Result<(), Box<dyn Error>> {
    info!("Launching server: {:?}", cmd);

    validate_opts(&cmd.opts)?;

    let server = builder::start_server(&cmd).await?;

    // Let server run to completion
    server.wait().await?;

    Ok(())
}

/// Enables using args for both human and non-human paths without cloning
macro_rules! format_content_println {
    ($format:expr, $content:expr, $human_expr:expr,) => {
        match $format {
            FormatOption::Human => {
                let result: Result<String, String> = $human_expr;
                match result {
                    Ok(x) => println!("{}", x),
                    Err(x) => eprintln!("{}", x),
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
            format_content_println!(
                cmd.format,
                Content::Version(x),
                Ok(x.version),
            )?;
        }
        client::Subcommand::Capabilities(_) => {
            let x = client.ask_capabilities().await?;
            format_content_println!(
                cmd.format,
                Content::Capabilities(x),
                Ok(format!("{:?}", x)),
            )?;
        }
        client::Subcommand::ListRootDir(_) => {
            let x = client.ask_list_root_dir_contents().await?;
            format_content_println!(
                cmd.format,
                Content::DirContentsList(x),
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
            format_content_println!(
                cmd.format,
                Content::DirContentsList(x),
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
            format_content_println!(
                cmd.format,
                Content::DirCreated(x),
                Ok(format!("Created {}", c.path)),
            )?;
        }
        client::Subcommand::MoveDir(c) => {
            let x = client.ask_rename_dir(c.from.clone(), c.to.clone()).await?;
            format_content_println!(
                cmd.format,
                Content::DirRenamed(x),
                Ok(format!("Moved {} to {}", c.from, c.to)),
            )?;
        }
        client::Subcommand::RemoveDir(c) => {
            let x = client.ask_remove_dir(c.path.clone(), c.non_empty).await?;
            format_content_println!(
                cmd.format,
                Content::DirRemoved(x),
                Ok(format!("Removed {}", c.path)),
            )?;
        }
        client::Subcommand::WriteFile(c) => {
            let mut file = client.ask_open_file(c.path.clone()).await?.into();
            let x = client
                .ask_write_file(&mut file, c.contents.as_ref())
                .await?;
            format_content_println!(
                cmd.format,
                Content::FileWritten(x),
                Ok(format!("{:?}", x)),
            )?;
        }
        client::Subcommand::ReadFile(c) => {
            let file = client.ask_open_file(c.path.clone()).await?.into();
            let x = client.ask_read_file(&file).await?;
            format_content_println!(
                cmd.format,
                Content::FileContents(x),
                Ok(String::from_utf8(x.contents)?),
            )?;
        }
        client::Subcommand::MoveFile(c) => {
            let x = client
                .ask_rename_unopened_file(c.from.clone(), c.to.clone())
                .await?;
            format_content_println!(
                cmd.format,
                Content::UnopenedFileRenamed(x),
                Ok(format!("Moved {} to {}", c.from, c.to)),
            )?;
        }
        client::Subcommand::RemoveFile(c) => {
            let x = client.ask_remove_unopened_file(c.path.clone()).await?;
            format_content_println!(
                cmd.format,
                Content::UnopenedFileRemoved(x),
                Ok(format!("Removed {}", c.path)),
            )?;
        }
        client::Subcommand::Exec(c) => {
            let proc = client
                .ask_exec_proc(c.command.clone(), c.args.clone())
                .await?
                .into();
            process_proc(
                client,
                c.stdin,
                c.post_exit_duration,
                proc,
                cmd.format,
                cmd.exit_print,
            )
            .await;
        }
        client::Subcommand::ReattachExec(c) => {
            let proc = RemoteProc::shallow(c.id);
            process_proc(
                client,
                c.stdin,
                c.post_exit_duration,
                proc,
                cmd.format,
                cmd.exit_print,
            )
            .await;
        }
        client::Subcommand::InternalDebug(_) => {
            let x = client.ask_internal_debug().await?;
            format_content_println!(
                cmd.format,
                Content::InternalDebug(x),
                Ok(format!("{}", String::from_utf8_lossy(&x.output))),
            )?;
        }
    };

    Ok(())
}

async fn process_proc(
    mut client: ConnectedClient,
    send_stdin: bool,
    post_exit_duration: Duration,
    proc: RemoteProc,
    format: FormatOption,
    exit_print: bool,
) {
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
                    .ask_write_stdin(&proc, &line.into_bytes())
                    .await
                    .expect("Failed to write stdin");
            }
        }

        let stdout_args = client
            .ask_get_stdout(&proc)
            .await
            .expect("Failed to get stdout");
        if !stdout_args.output.is_empty() {
            format_content_println!(
                format,
                Content::StdoutContents(stdout_args),
                Ok(format!("{}", String::from_utf8_lossy(&stdout_args.output))),
            )
            .expect("Failed to format stdout");
        }

        let stderr_args = client
            .ask_get_stderr(&proc)
            .await
            .expect("Failed to get stderr");
        if !stderr_args.output.is_empty() {
            format_content_println!(
                format,
                Content::StderrContents(stderr_args),
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
                .ask_proc_status(&proc)
                .await
                .expect("Failed to get proc status");
            if !status.is_alive {
                match format {
                    FormatOption::Human if exit_print => {
                        eprintln!(
                            "Proc {} exited with code {}",
                            status.id,
                            status.exit_code.unwrap_or_default(),
                        );
                        Ok(())
                    }
                    FormatOption::Human => Ok(()),
                    f => format::format_content_println(
                        f,
                        Content::ProcStatus(status),
                        |_| Err("unreachable!".into()),
                    ),
                }
                .expect("Failed to format status");
                exit_instant = Some(Instant::now());
            }
        }
    }
}
