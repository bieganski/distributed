use std::io::{Read, Write};
use std::marker::PhantomData;
use rustls::{NoClientAuth};
use std::sync::Arc;

// PhantomData marker is here only to remove `unused type parameter` error.
// Your final solution should not need it.

pub struct SecureClient<L: Read + Write> {
    // config: rustls::ClientConfig,
    conn: rustls::StreamOwned<rustls::ClientSession, L>,
    // mac_provider: & dyn MacKeyProvider,
}

pub struct SecureServer<L: Read + Write> {
    // phantom: PhantomData<L>,
    conn: rustls::StreamOwned<rustls::ServerSession, L>,
}

use crate::keys::{ROOT_CERT, SERVER_FULL_CHAIN, SERVER_PRIVATE_KEY};
impl<L: Read + Write> SecureClient<L> {
    pub fn new(link: L, hmac_key_provider: &dyn MacKeyProvider, root_cert: &str) -> Self {

        let mut config = rustls::ClientConfig::new();
        config
            .root_store
            .add_pem_file(&mut root_cert.as_bytes())
            .unwrap();
        let dns_name = webpki::DNSNameRef::try_from_ascii_str("localhost").unwrap();
        let sess = rustls::ClientSession::new(&Arc::new(config), dns_name);
        let conn = rustls::StreamOwned::new(sess, link);

        let res = SecureClient{conn: conn}; // mac_provider: hmac_key_provider
        res
    }

    pub fn send_msg(&mut self, _data: Vec<u8>) {
        unimplemented!()
    }
}

impl<L: Read + Write> SecureServer<L> {
    pub fn new(
        link: L,
        _hmac_key_provider: &dyn MacKeyProvider,
        server_private_key: &str,
        server_full_chain: &str,
    ) -> Self {
        // TODO NoClientAuth
        let mut config = rustls::ServerConfig::new(NoClientAuth::new());
        let certs = rustls::internal::pemfile::certs(&mut server_full_chain.as_bytes()).unwrap();
        let private_key = rustls::internal::pemfile::rsa_private_keys(
            &mut server_private_key.as_bytes()
        ).unwrap().remove(0);

        config.set_single_cert(certs, private_key).unwrap();
        let conn = rustls::ServerSession::new(&Arc::new(config));

        let res = SecureServer{conn: rustls::StreamOwned::new(conn, link)};
        res
    }

    /// Returns next unencrypted message with HMAC tag at the end
    pub fn recv_message(&mut self) -> Result<Vec<u8>, SecureServerError> {
        unimplemented!()
    }
}

pub trait MacKeyProvider {
    fn key(&self) -> &[u8];
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SecureServerError {
    InvalidHmac,
}
