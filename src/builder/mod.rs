mod auth;
mod crypto;

use crate::opts::{client::ClientCommand, server::ServerCommand, types};
use over_there_core::{
    ClientBuilder, ConnectedClient, ListeningServer, ServerBuilder, Transport,
};
use over_there_wire::{Authenticator, Bicrypter};
use std::io;

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
    A: Authenticator + Send + Clone + Default + 'static,
    B: Bicrypter + Send + Clone + Default + 'static,
{
    let internal_buffer_size = cmd.opts.internal_buffer_size;
    let packet_ttl = cmd.opts.packet_ttl;
    let addrs = vec![cmd.addr];
    let transport = match cmd.opts.transport {
        types::Transport::Tcp => Transport::Tcp(addrs),
        types::Transport::Udp => Transport::Udp(addrs),
    };

    ClientBuilder::default()
        .authenticator(authenticator)
        .bicrypter(bicrypter)
        .transport(transport)
        .buffer(internal_buffer_size)
        .packet_ttl(packet_ttl)
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
    A: Authenticator + Send + Clone + Default + 'static,
    B: Bicrypter + Send + Clone + Default + 'static,
{
    let internal_buffer_size = cmd.opts.internal_buffer_size;
    let packet_ttl = cmd.opts.packet_ttl;
    let addrs = over_there_wire::net::make_addr_list(
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
        .buffer(internal_buffer_size)
        .packet_ttl(packet_ttl);

    if !cmd.no_root {
        config.root(cmd.root_or_default());
    }

    config
        .build()
        .map_err(|x| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid server config: {}", x),
            )
        })?
        .listen()
        .await
}
