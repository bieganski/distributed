use crossbeam_channel::Sender;
use std::io::{stdout, Read, Write};
use std::net::TcpListener;

pub fn echo_once(server_up_notify: Sender<()>) {
    let socket = TcpListener::bind("127.0.0.1:8889").unwrap();
    server_up_notify.send(()).unwrap();

    for stream in socket.incoming().take(1) {
        let mut buf = Vec::new();
        let mut stream = stream.unwrap();

        // Receive the data.
        stream.read_to_end(&mut buf).unwrap();

        stdout().write_all(b"Server > received: ").unwrap();
        stdout().write_all(buf.as_ref()).unwrap();

        // Reply to the client.
        stream.write_all(&buf).unwrap();
    }
}
