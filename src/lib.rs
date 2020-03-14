mod builder;
mod opts;

use log::info;
use opts::{
    client::{self, ClientCommand},
    server::ServerCommand,
    Command,
};
use std::error::Error;
use std::io;

pub use opts::Opts;

/// Primary entrypoint to run the executable based on input options
pub async fn run(opts: Opts) -> Result<(), Box<dyn Error>> {
    match opts.command {
        Command::Server(s) => run_server(s).await?,
        Command::Client(c) => run_client(c).await?,
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

async fn run_client(cmd: ClientCommand) -> Result<(), Box<dyn Error>> {
    info!("Launching client: {:?}", cmd);

    validate_opts(&cmd.opts)?;

    let mut client = builder::start_client(&cmd)
        .await
        .expect("Failed to connect with client");

    match &cmd.command {
        client::Subcommand::Version(_) => {
            println!("{}", client.ask_version().await?)
        }
        client::Subcommand::Capabilities(_) => {
            println!("{:?}", client.ask_capabilities().await?)
        }
        client::Subcommand::RootDir(_) => println!(
            "{}",
            client
                .ask_list_root_dir_contents()
                .await?
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
                .join("\n")
        ),
        client::Subcommand::Dir(c) => {
            println!(
                "{}",
                client
                    .ask_list_dir_contents(c.path.clone())
                    .await?
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
                    .join("\n")
            );
        }
        client::Subcommand::WriteFile(c) => {
            let mut file = client.ask_open_file(c.path.clone()).await?;
            println!(
                "{:?}",
                client.ask_write_file(&mut file, c.contents.as_ref()).await
            );
        }
        client::Subcommand::ReadFile(c) => {
            let file = client.ask_open_file(c.path.clone()).await?;
            let bytes = client.ask_read_file(&file).await?;
            println!(
                "{}",
                String::from_utf8(bytes)
                    .expect("Failed to translate file back to string")
            );
        }
        client::Subcommand::Exec(c) => {
            let proc = client
                .ask_exec_proc(c.command.clone(), c.args.clone())
                .await?;

            let stdin = io::stdin();

            loop {
                if c.stdin {
                    use io::BufRead;
                    let mut handle = stdin.lock();
                    let mut line = String::new();
                    match handle.read_line(&mut line) {
                        Ok(_) => (),
                        Err(x) if x.kind() == io::ErrorKind::WouldBlock => (),
                        Err(x) => {
                            panic!("Failed to read line of input: {:?}", x)
                        }
                    }

                    if !line.is_empty() {
                        client
                            .ask_write_stdin(&proc, &line.into_bytes())
                            .await
                            .expect("Failed to write stdin");
                    }
                }

                let contents = client
                    .ask_get_stdout(&proc)
                    .await
                    .expect("Failed to get stdout");
                if !contents.is_empty() {
                    println!("{}", String::from_utf8_lossy(&contents));
                }

                let contents = client
                    .ask_get_stderr(&proc)
                    .await
                    .expect("Failed to get stderr");
                if !contents.is_empty() {
                    eprintln!("{}", String::from_utf8_lossy(&contents));
                }

                // Exit the loop if the proc has exited
                let status = client
                    .ask_proc_status(&proc)
                    .await
                    .expect("Failed to get proc status");
                if !status.is_alive {
                    break;
                }
            }
        }
        client::Subcommand::ReattachExec(_c) => unimplemented!(),
        client::Subcommand::InternalDebug(_) => println!(
            "{}",
            String::from_utf8_lossy(&client.ask_internal_debug().await?)
        ),
    };

    Ok(())
}
