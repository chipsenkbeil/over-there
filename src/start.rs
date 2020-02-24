use super::opts::{
    types::{self, Authentication, Encryption},
    CommonOpts,
};
use over_there_auth::{self as auth};
use over_there_core::{Client, Communicator, Server, Transport};
use over_there_crypto::{self as crypto, key::Key};
use std::io;
use std::net::SocketAddr;

macro_rules! start_communicator {
    (
        $new_bicrypter_func:expr, 
        $new_authenticator_func:expr, 
        $packet_ttl:ident, 
        $transport:ident, 
        $internal_buffer_size:ident, 
        $method:ident
    ) => {{
        let bicrypter = $new_bicrypter_func;
        let authenticator = $new_authenticator_func;
        Communicator::new($packet_ttl, authenticator, bicrypter)
            .$method($transport, $internal_buffer_size)
            .await
    }};
}

macro_rules! match_key_or_err {
    ($key_enum:path, $key_str:expr) => {{
        let empty_str = String::new();
        if let Some($key_enum(key)) =
            Key::from_slice($key_str.as_ref().unwrap_or(&empty_str).as_bytes())
        {
            Ok(key)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Provided encryption key not right length",
            ))
        }
    }};
}

macro_rules! build_and_start_from_opts {
    ($opts:expr, $addrs:expr, $method:ident) => {{
        let empty_str = String::new();
        let akey_str = &$opts.authentication_key.as_ref().unwrap_or(&empty_str);
        let akey = akey_str.as_bytes();
        let internal_buffer_size = $opts.internal_buffer_size;
        let packet_ttl = $opts.packet_ttl;

        let transport = match $opts.transport {
            types::Transport::Tcp => Transport::Tcp($addrs),
            types::Transport::Udp => Transport::Udp($addrs),
        };

        match ($opts.encryption, $opts.authentication) {
            (Encryption::AesGcm128, Authentication::None) => {
                let key =
                    match_key_or_err!(Key::Key128Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm::new_aes_128_gcm_bicrypter(&key),
                    auth::NoopAuthenticator,
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcm256, Authentication::None) => {
                let key =
                    match_key_or_err!(Key::Key256Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm::new_aes_256_gcm_bicrypter(&key),
                    auth::NoopAuthenticator,
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcmSiv128, Authentication::None) => {
                let key =
                    match_key_or_err!(Key::Key128Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm_siv::new_aes_128_gcm_siv_bicrypter(&key),
                    auth::NoopAuthenticator,
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcmSiv256, Authentication::None) => {
                let key =
                    match_key_or_err!(Key::Key256Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm_siv::new_aes_256_gcm_siv_bicrypter(&key),
                    auth::NoopAuthenticator,
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcm128, Authentication::Sha256) => {
                let key =
                    match_key_or_err!(Key::Key128Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm::new_aes_128_gcm_bicrypter(&key),
                    auth::Sha256Authenticator::new(akey),
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcm256, Authentication::Sha256) => {
                let key =
                    match_key_or_err!(Key::Key256Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm::new_aes_256_gcm_bicrypter(&key),
                    auth::Sha256Authenticator::new(akey),
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcmSiv128, Authentication::Sha256) => {
                let key =
                    match_key_or_err!(Key::Key128Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm_siv::new_aes_128_gcm_siv_bicrypter(&key),
                    auth::Sha256Authenticator::new(akey),
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcmSiv256, Authentication::Sha256) => {
                let key =
                    match_key_or_err!(Key::Key256Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm_siv::new_aes_256_gcm_siv_bicrypter(&key),
                    auth::Sha256Authenticator::new(akey),
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcm128, Authentication::Sha512) => {
                let key =
                    match_key_or_err!(Key::Key128Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm::new_aes_128_gcm_bicrypter(&key),
                    auth::Sha512Authenticator::new(akey),
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcm256, Authentication::Sha512) => {
                let key =
                    match_key_or_err!(Key::Key256Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm::new_aes_256_gcm_bicrypter(&key),
                    auth::Sha512Authenticator::new(akey),
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcmSiv128, Authentication::Sha512) => {
                let key =
                    match_key_or_err!(Key::Key128Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm_siv::new_aes_128_gcm_siv_bicrypter(&key),
                    auth::Sha512Authenticator::new(akey),
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::AesGcmSiv256, Authentication::Sha512) => {
                let key =
                    match_key_or_err!(Key::Key256Bits, $opts.encryption_key)?;
                start_communicator!(
                    crypto::aes_gcm_siv::new_aes_256_gcm_siv_bicrypter(&key),
                    auth::Sha512Authenticator::new(akey),
                    packet_ttl,
                    transport,
                    internal_buffer_size,
                    $method
                )
            }
            (Encryption::None, Authentication::Sha256) => start_communicator!(
                crypto::NoopBicrypter,
                auth::Sha256Authenticator::new(akey),
                packet_ttl,
                transport,
                internal_buffer_size,
                $method
            ),
            (Encryption::None, Authentication::Sha512) => start_communicator!(
                crypto::NoopBicrypter,
                auth::Sha512Authenticator::new(akey),
                packet_ttl,
                transport,
                internal_buffer_size,
                $method
            ),
            (Encryption::None, Authentication::None) => start_communicator!(
                crypto::NoopBicrypter,
                auth::NoopAuthenticator,
                packet_ttl,
                transport,
                internal_buffer_size,
                $method
            ),
        }
    }};
}

pub async fn start_client(
    opts: &CommonOpts,
    addr: SocketAddr,
) -> Result<Client, io::Error> {
    #![allow(clippy::cognitive_complexity)]
    let addrs = vec![addr];
    build_and_start_from_opts!(opts, addrs, connect)
}

pub async fn start_server(
    opts: &CommonOpts,
    addrs: Vec<SocketAddr>,
) -> Result<Server, io::Error> {
    #![allow(clippy::cognitive_complexity)]
    build_and_start_from_opts!(opts, addrs, listen)
}
