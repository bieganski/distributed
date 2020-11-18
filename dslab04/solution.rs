use std::io::{Read, Write};
use std::marker::PhantomData;

// PhantomData marker is here only to remove `unused type parameter` error.
// Your final solution should not need it.

pub struct SecureClient<L: Read + Write> {
    phantom: PhantomData<L>,
}

pub struct SecureServer<L: Read + Write> {
    phantom: PhantomData<L>,
}

impl<L: Read + Write> SecureClient<L> {
    pub fn new(_link: L, _hmac_key_provider: &dyn MacKeyProvider, _root_cert: &str) -> Self {
        unimplemented!()
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
