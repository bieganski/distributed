// The random number generation is not in std.
use rand::rngs::OsRng;
use rand::Rng;

// For RSA, there is a special crate.
use rsa::{PaddingScheme, PublicKey, RSAPrivateKey};

// Hmac uses also sha2 crate.
use hmac::{Hmac, Mac, NewMac};
use sha2::Sha256;

// AES is provided via a software implementation.
// Hardware-accelerated implementations shall perform the operations always in
// the same number of processor cycles – remember this remark if one day you will
// feel like implementing your own non-toy cryptography. This task is full of pitfalls!
use aes_soft::Aes128;

// Block mode for AES to encrypt large data chunks.
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};

fn rsa_cryptography() {
    // Generating 256bit RSA key.
    let rsa_key = RSAPrivateKey::new(&mut OsRng, 256).expect("Failed to generate a key!");

    // The message.
    let msg = "Secret message";

    // Let's encrypt it. For encryption, the public key is enough.
    let encrypted = rsa_key
        .to_public_key()
        .encrypt(&mut OsRng, PaddingScheme::PKCS1v15, msg.as_bytes())
        .unwrap();
    println!("RSA encrypted data: {:?}", encrypted);

    // Let's decrypt.
    let decrypted = rsa_key
        .decrypt(PaddingScheme::PKCS1v15, encrypted.as_ref())
        .map(String::from_utf8)
        .unwrap()
        .unwrap();
    assert_eq!(decrypted, msg)
}

type Aes128Cbc = Cbc<Aes128, Pkcs7>;

fn aes_cryptography() {
    // The message.
    let message = "AES is fast for large amounts of data";

    // Generate key and initialization vector.
    let key = rand::thread_rng().gen::<[u8; 16]>();
    let iv = rand::thread_rng().gen::<[u8; 16]>();

    // Let's encrypt.
    let cipher = Aes128Cbc::new_var(&key, &iv).unwrap();
    let encrypted = cipher.encrypt_vec(message.as_bytes());
    println!("AES encrypted data: {:?}", encrypted);

    // Let's decrypt.
    let cipher = Aes128Cbc::new_var(&key, &iv).unwrap();
    let decrypted = cipher
        .decrypt_vec(encrypted.as_ref())
        .map(String::from_utf8)
        .unwrap()
        .unwrap();
    assert_eq!(decrypted, message)
}

type HmacSha256 = Hmac<Sha256>;

fn hmac() {
    // The message.
    let msg = "Message requiring authorization";

    // Let's generate the tag.
    let mut mac = HmacSha256::new_varkey(&[1, 2, 3]).expect("HMAC can take key of any size");
    mac.update(msg.as_bytes());
    let tag = mac.finalize().into_bytes();
    println!("Length of hmac tag: {}", tag.len());

    // Let's verify the tag.
    let mut mac = HmacSha256::new_varkey(&[1, 2, 3]).expect("HMAC can take key of any size");
    mac.update(msg.as_bytes());
    assert!(mac.verify(tag.as_ref()).is_ok());
}

fn main() {
    rsa_cryptography();
    aes_cryptography();
    hmac();
}
