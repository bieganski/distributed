use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};

pub fn send_msg() {
    // There is also useful `connect_timeout`.
    // Using IPv6 address in `connect` will use IPv6 for transport.
    let mut stream = TcpStream::connect("127.0.0.1:8889").unwrap();

    // Send a message.
    let msg = "This is test\n";
    print!("Client > sending: {}", msg);
    stream.write_all(msg.as_bytes()).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();

    // Receive a reply.
    let mut buf = Vec::new();
    // A reference to vector is passed to `read_to_end`, which
    // will allocate memory as needed to store the bytes.
    stream.read_to_end(&mut buf).unwrap();
    print!(
        "Client > received: {}",
        std::string::String::from_utf8(buf).unwrap()
    );
}
