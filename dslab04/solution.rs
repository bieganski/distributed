use std::io::{Read, Write};
use std::marker::PhantomData;
use rustls::{NoClientAuth};
use std::sync::Arc;

// PhantomData marker is here only to remove `unused type parameter` error.
// Your final solution should not need it.

pub struct SecureClient<L: Read + Write> {
    conn: rustls::StreamOwned<rustls::ClientSession, L>,
    key : Vec<u8>,
}

pub struct SecureServer<L: Read + Write> {
    conn: rustls::StreamOwned<rustls::ServerSession, L>,
    key : Vec<u8>,
}

impl<L: Read + Write> SecureClient<L> {
    pub fn new(link: L, hmac_key_provider: &dyn MacKeyProvider, root_cert: &str) -> Self {

        let mut config = rustls::ClientConfig::new();
        config
            .root_store
            .add_pem_file(&mut root_cert.as_bytes())
            .unwrap();
        let dns_name = webpki::DNSNameRef::try_from_ascii_str("localhost").unwrap();
        let sess = rustls::ClientSession::new(&Arc::new(config), dns_name);
        let key = (*hmac_key_provider.key()).to_vec();

        let res = SecureClient{conn: rustls::StreamOwned::new(sess, link), key : key}; // mac_provider: hmac_key_provider
        res
    }

    pub fn send_msg(&mut self, data: Vec<u8>) {
        println!("trying to send: {:?}", data);
        self.conn.write(data.as_slice()).unwrap();
        // self.conn.write_all(data.as_slice()).unwrap(); // TODO error handling
        print!("data written: {}", std::str::from_utf8(data.as_ref()).unwrap());
    }
}

impl<L: Read + Write> SecureServer<L> {
    pub fn new(
        link: L,
        hmac_key_provider: &dyn MacKeyProvider,
        server_private_key: &str,
        server_full_chain: &str,
    ) -> Self {
        let mut config = rustls::ServerConfig::new(NoClientAuth::new());
        let certs = rustls::internal::pemfile::certs(&mut server_full_chain.as_bytes()).unwrap();
        let private_key = rustls::internal::pemfile::rsa_private_keys(
            &mut server_private_key.as_bytes()
        ).unwrap().remove(0);

        config.set_single_cert(certs, private_key).unwrap();
        let sess = rustls::ServerSession::new(&Arc::new(config));

        let key = (*hmac_key_provider.key()).to_vec();

        let res = SecureServer{conn: rustls::StreamOwned::new(sess, link), key : key};
        res
    }

    /// Returns next unencrypted message with HMAC tag at the end
    pub fn recv_message(&mut self) -> Result<Vec<u8>, SecureServerError> {
        let mut data  = [0_u8; 5];
        println!("trying to read data...");
        // self.conn.read_to_end(&mut data).unwrap(); // TODO inne ready
        self.conn.read_exact(&mut data).unwrap(); // TODO inne ready
        // self.conn.rea
        print!("data read: {}", std::str::from_utf8(data.as_ref()).unwrap());
        // Ok(vec!(data))
        Err(SecureServerError::InvalidHmac)
    }
}

pub trait MacKeyProvider {
    fn key(&self) -> &[u8];
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SecureServerError {
    InvalidHmac,
}
