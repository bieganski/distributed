use std::net::UdpSocket;
use std::time::Duration;

pub fn client() {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let count = socket
        .send_to(b"testing UTF-8 string", "127.0.0.1:8889")
        .unwrap();
    println!("Client > sent {} bytes!", count);

    let mut buf = [0; 32];

    let count = socket.recv(&mut buf).unwrap();
    println!(
        "Client > received: {}",
        std::str::from_utf8(&buf[..count]).unwrap()
    );

    // `connect` sets a default target for `send` method.
    socket.connect("127.0.0.1:8889").unwrap();
    let count = socket.send(b"using connect").unwrap();
    println!("Client > sent {} bytes!", count);
    socket
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let count = socket.recv(&mut buf).unwrap();
    println!(
        "Client > received: {}",
        std::str::from_utf8(&buf[..count]).unwrap()
    );
}
