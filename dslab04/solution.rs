use std::io::{Read, Write};
use std::marker::PhantomData;
use rustls;
use std::sync::Arc;

// PhantomData marker is here only to remove `unused type parameter` error.
// Your final solution should not need it.

pub struct SecureClient<L: Read + Write> {
    config: rustls::ClientConfig,
    conn: rustls::StreamOwned<rustls::ClientSession, L>
}

pub struct SecureServer<L: Read + Write> {
    phantom: PhantomData<L>,
}

use crate::keys::{ROOT_CERT, SERVER_FULL_CHAIN, SERVER_PRIVATE_KEY};
impl<L: Read + Write> SecureClient<L> {
    pub fn new(link: L, _hmac_key_provider: &dyn MacKeyProvider, _root_cert: &str) -> Self {
        unimplemented!();

        let mut config = rustls::ClientConfig::new();
        config
            .root_store
            .add_pem_file(&mut ROOT_CERT.as_bytes())
            .unwrap();
        let dns_name = webpki::DNSNameRef::try_from_ascii_str("localhost").unwrap();
        let sess = rustls::ClientSession::new(&Arc::new(config), dns_name);
        let conn = rustls::StreamOwned::new(sess, link);

        let res = SecureClient{config : config, conn: conn};
        res
    }

    pub fn send_msg(&mut self, _data: Vec<u8>) {
        unimplemented!()
    }
}

impl<L: Read + Write> SecureServer<L> {
    pub fn new(
        _link: L,
        _hmac_key_provider: &dyn MacKeyProvider,
        _server_private_key: &str,
        _server_full_chain: &str,
    ) -> Self {
        unimplemented!()
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
