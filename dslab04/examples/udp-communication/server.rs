use crossbeam_channel::Sender;
use std::net::UdpSocket;

pub fn echo(server_ready_notify: Sender<()>) {
    let socket = UdpSocket::bind("127.0.0.1:8889").unwrap();
    server_ready_notify.send(()).unwrap();

    let mut buf = [0; 20];

    for _ in 0..2 {
        let (count, addr) = socket.recv_from(&mut buf).unwrap();
        println!(
            "Server > received {} bytes! Message: {}",
            count,
            std::str::from_utf8(&buf[..count]).unwrap()
        );
        socket.send_to(&buf[..count], addr).unwrap();
    }
}
