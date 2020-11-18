use std::io::{Read, Write};
use rustls::{NoClientAuth};
use std::sync::Arc;

use hmac::{Hmac, NewMac, Mac};
use sha2::Sha256;
use std::convert::TryInto;


type HmacSha256 = Hmac<Sha256>;

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
        // let mut send_data = vec![0_u8; MSG_LEN]; // data.len() + MSG_LEN + HMAC_LEN
        let mut msg = (data.len() as u32).to_be_bytes().to_vec(); // .to_be(); // u32::from(data.len());
        msg.extend(&data);
        // msg_len.extend();
        let mut mac = HmacSha256::new_varkey(&self.key).expect("HMAC can take key of any size");
        mac.update(msg.as_slice());
        let tag = mac.finalize().into_bytes();
        msg.extend(&tag);
        println!("t2s2: {:?}", msg);
        // send_data.extend_from_slice(&[0_u8; HMAC_LEN]);


        self.conn.write(msg.as_slice()).unwrap();
        // self.conn.write_all(data.as_slice()).unwrap(); // TODO error handling
        // print!("data written: {}", std::str::from_utf8(data.as_ref()).unwrap());
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
        let mut data = [0_u8; 100];
        // let mut data  = [0_u8; 50].to_vec();
        println!("trying to read data...");
        let total_len : u32 = self.conn.read(&mut data).unwrap() as u32;
        let msg_size = u32::from_be_bytes((&data[0..4]).try_into().expect("no way"));
        assert_eq!(msg_size, total_len - 4 - 32); // HMAC tag (32 bytes) + 4 MSG_SIZE (4 bytes)
        // let msg_size = u32::from_be_bytes(array_ref!(data, 0, 4));
        
        // parse message
        let mut mac = HmacSha256::new_varkey(&self.key).expect("HMAC can take key of any size");
        mac.update(&data[0..(msg_size as usize)+4]);
        let tag = mac.finalize().into_bytes();

        let msg_tag = &data[4 + msg_size as usize..4+32+msg_size as usize];
        // println!("TAGGE: {:?}", tag);
        // println!("TAGGE: {:?}", &msg_tag);

        let tag_vec = tag.to_vec();
        let msg_tag_vec = msg_tag.to_vec();
        println!("TAGGE: {:?}", &tag_vec);
        println!("TAGGE: {:?}", &msg_tag_vec);
        if (tag_vec == msg_tag_vec) {
            print!("TAK!");
        } else {
            print!("NIE!");
        }

        // self.conn.read_to_end(&mut data).unwrap(); // TODO inne ready
        // self.conn.read_exact(&mut data).unwrap(); // TODO inne ready
        // self.conn.rea
        print!("data read: {}", std::str::from_utf8(data.as_ref()).unwrap());
        Ok(data.to_vec())
        // Err(SecureServerError::InvalidHmac)
    }
}

pub trait MacKeyProvider {
    fn key(&self) -> &[u8];
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SecureServerError {
    InvalidHmac,
}
