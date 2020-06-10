mod auth;
mod crypto;

use crate::cli::opts::{client::ClientCommand, server::ServerCommand, types};
use log::debug;
use crate::core::{
    ClientBuilder, ConnectedClient, ListeningServer, ServerBuilder, Transport,
};
use crate::core::transport::{Authenticator, Bicrypter};
use std::io;
use tokio::net;

pub async fn start_client(cmd: &ClientCommand) -> io::Result<ConnectedClient> {
    match (
        auth::Authenticator::new(
            cmd.opts.authentication,
            cmd.opts.authentication_key.clone(),
        )?,
        crypto::Bicrypter::new(
            cmd.opts.encryption,
            cmd.opts.encryption_key.clone(),
        )?,
    ) {
        (auth::Authenticator::None(a), crypto::Bicrypter::None(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::Sha256(a), crypto::Bicrypter::None(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::Sha512(a), crypto::Bicrypter::None(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::None(a), crypto::Bicrypter::Aes128Gcm(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::Sha256(a), crypto::Bicrypter::Aes128Gcm(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::Sha512(a), crypto::Bicrypter::Aes128Gcm(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::None(a), crypto::Bicrypter::Aes128GcmSiv(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (
            auth::Authenticator::Sha256(a),
            crypto::Bicrypter::Aes128GcmSiv(b),
        ) => build_client_and_connect(cmd, a, b).await,
        (
            auth::Authenticator::Sha512(a),
            crypto::Bicrypter::Aes128GcmSiv(b),
        ) => build_client_and_connect(cmd, a, b).await,
        (auth::Authenticator::None(a), crypto::Bicrypter::Aes256Gcm(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::Sha256(a), crypto::Bicrypter::Aes256Gcm(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::Sha512(a), crypto::Bicrypter::Aes256Gcm(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (auth::Authenticator::None(a), crypto::Bicrypter::Aes256GcmSiv(b)) => {
            build_client_and_connect(cmd, a, b).await
        }
        (
            auth::Authenticator::Sha256(a),
            crypto::Bicrypter::Aes256GcmSiv(b),
        ) => build_client_and_connect(cmd, a, b).await,
        (
            auth::Authenticator::Sha512(a),
            crypto::Bicrypter::Aes256GcmSiv(b),
        ) => build_client_and_connect(cmd, a, b).await,
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unsupported authentication or encryption protocol",
        )),
    }
}

async fn build_client_and_connect<A, B>(
    cmd: &ClientCommand,
    authenticator: A,
    bicrypter: B,
) -> io::Result<ConnectedClient>
where
    A: Authenticator + Send + Sync + Clone + Default + 'static,
    B: Bicrypter + Send + Sync + Clone + Default + 'static,
{
    // Attempt to resolve provided address, filtering out IPv4 if looking for
    // IPv6 and vice versa, selecting very first match in resolution
    let maybe_resolved_addr = net::lookup_host(cmd.addr.clone())
        .await?
        .find(|x| x.is_ipv6() == cmd.ipv6);

    debug!(
        "Resolved {} to {}",
        cmd.addr,
        maybe_resolved_addr
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or_default()
    );

    let addrs = maybe_resolved_addr.map(|x| vec![x]).unwrap_or_default();
    let transport = match cmd.opts.transport {
        types::Transport::Tcp => Transport::Tcp(addrs),
        types::Transport::Udp => Transport::Udp(addrs),
    };

    ClientBuilder::default()
        .authenticator(authenticator)
        .bicrypter(bicrypter)
        .transport(transport)
        .buffer(cmd.opts.internal_buffer_size)
        .packet_ttl(cmd.opts.packet_ttl)
        .build()
        .map_err(|x| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid client config: {}", x),
            )
        })?
        .connect()
        .await
}

pub async fn start_server(cmd: &ServerCommand) -> io::Result<ListeningServer> {
    match (
        auth::Authenticator::new(
            cmd.opts.authentication,
            cmd.opts.authentication_key.clone(),
        )?,
        crypto::Bicrypter::new(
            cmd.opts.encryption,
            cmd.opts.encryption_key.clone(),
        )?,
    ) {
        (auth::Authenticator::None(a), crypto::Bicrypter::None(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::Sha256(a), crypto::Bicrypter::None(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::Sha512(a), crypto::Bicrypter::None(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::None(a), crypto::Bicrypter::Aes128Gcm(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::Sha256(a), crypto::Bicrypter::Aes128Gcm(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::Sha512(a), crypto::Bicrypter::Aes128Gcm(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::None(a), crypto::Bicrypter::Aes128GcmSiv(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (
            auth::Authenticator::Sha256(a),
            crypto::Bicrypter::Aes128GcmSiv(b),
        ) => build_server_and_listen(cmd, a, b).await,
        (
            auth::Authenticator::Sha512(a),
            crypto::Bicrypter::Aes128GcmSiv(b),
        ) => build_server_and_listen(cmd, a, b).await,
        (auth::Authenticator::None(a), crypto::Bicrypter::Aes256Gcm(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::Sha256(a), crypto::Bicrypter::Aes256Gcm(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::Sha512(a), crypto::Bicrypter::Aes256Gcm(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (auth::Authenticator::None(a), crypto::Bicrypter::Aes256GcmSiv(b)) => {
            build_server_and_listen(cmd, a, b).await
        }
        (
            auth::Authenticator::Sha256(a),
            crypto::Bicrypter::Aes256GcmSiv(b),
        ) => build_server_and_listen(cmd, a, b).await,
        (
            auth::Authenticator::Sha512(a),
            crypto::Bicrypter::Aes256GcmSiv(b),
        ) => build_server_and_listen(cmd, a, b).await,
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unsupported authentication or encryption protocol",
        )),
    }
}

async fn build_server_and_listen<A, B>(
    cmd: &ServerCommand,
    authenticator: A,
    bicrypter: B,
) -> io::Result<ListeningServer>
where
    A: Authenticator + Send + Sync + Clone + Default + 'static,
    B: Bicrypter + Send + Sync + Clone + Default + 'static,
{
    let addrs = crate::core::transport::net::make_addr_list(
        cmd.addr.ip(),
        vec![cmd.addr.port()],
    );
    let transport = match cmd.opts.transport {
        types::Transport::Tcp => Transport::Tcp(addrs),
        types::Transport::Udp => Transport::Udp(addrs),
    };

    let mut config = ServerBuilder::default();

    config
        .authenticator(authenticator)
        .bicrypter(bicrypter)
        .transport(transport)
        .cleanup_interval(cmd.cleanup_interval)
        .file_ttl(cmd.untouched_file_ttl)
        .proc_ttl(cmd.untouched_proc_ttl)
        .dead_proc_ttl(cmd.dead_proc_ttl)
        .buffer(cmd.opts.internal_buffer_size)
        .packet_ttl(cmd.opts.packet_ttl);

    // Change our process's current working directory if specified
    if let Some(path) = cmd.working_dir.as_ref() {
        debug!("Server working dir: {}", path.to_string_lossy().to_string());
        std::env::set_current_dir(path)?;
    }

    config
        .build()
        .map_err(|x| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid server config: {}", x),
            )
        })?
        .cloneable_listen()
        .await
}
