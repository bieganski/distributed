use std::io::{Read, Write};
use rustls::{NoClientAuth};
use std::sync::Arc;

use hmac::{Hmac, NewMac, Mac};
use sha2::Sha256;
use std::convert::TryInto;

type HmacSha256 = Hmac<Sha256>;

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

        let res = SecureClient{conn: rustls::StreamOwned::new(sess, link), key : key};
        res
    }

    pub fn send_msg(&mut self, data: Vec<u8>) {
        let mut msg = (data.len() as u32).to_be_bytes().to_vec();
        msg.extend(&data);
        let mut mac = HmacSha256::new_varkey(&self.key).expect("HMAC can take key of any size");
        mac.update(msg.as_slice());
        let tag = mac.finalize().into_bytes();
        msg.extend(&tag);
        self.conn.write_all(msg.as_slice()).unwrap();
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
        const MSG_SIZE_MAX : usize = 1000;
        let mut msg_content_buf = [0_u8; MSG_SIZE_MAX];
        // let total_len : u32 = self.conn.read(&mut data).unwrap() as u32;
        let mut msg_size_buf = [0_u8; 4];
        let mut msg_hmac_buf = [0_u8; 32];

        self.conn.read_exact(&mut msg_size_buf[0..]).unwrap();
        let msg_size = u32::from_be_bytes((&msg_size_buf[0..]).try_into().expect("no way"));

        self.conn.read_exact(&mut msg_content_buf[0..msg_size as usize]).unwrap();
        
        self.conn.read_exact(&mut msg_hmac_buf[0..]).unwrap();

        // parse message
        let mut mac = HmacSha256::new_varkey(&self.key).expect("HMAC can take key of any size");
        let mut concat = msg_size_buf.to_vec();
        concat.extend(&msg_content_buf[0..msg_size as usize]);

        mac.update(&concat[..]);
        let tag = mac.finalize().into_bytes();

        let tag_vec = tag.to_vec();
        let msg_tag_vec = msg_hmac_buf.to_vec();
        
        if tag_vec == msg_tag_vec {
            Ok(msg_content_buf[0..msg_size as usize].to_vec())
        } else {
            Err(SecureServerError::InvalidHmac)
        }
    }
}

pub trait MacKeyProvider {
    fn key(&self) -> &[u8];
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SecureServerError {
    InvalidHmac,
}
