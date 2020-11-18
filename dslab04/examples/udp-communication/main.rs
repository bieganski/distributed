mod client;
mod server;

use crossbeam_channel::unbounded;
use std::net::{SocketAddr, UdpSocket};

fn bind_interface() {
    {
        // Bind requires a value that implements trait ToSocketAddrs.
        let addr1 = SocketAddr::from(([0, 0, 0, 0], 8877));
        UdpSocket::bind(addr1).unwrap();

        // Port number zero means 'any port number'.
        let addr2 = SocketAddr::from(([127, 0, 0, 1], 0));
        let addrs = [addr1, addr2];

        // ToSocketAddrs may return multiple addresses.
        UdpSocket::bind(&addrs[..]).unwrap();
    }
}

fn socket_api() {
    let any_local_port = SocketAddr::from(([127, 0, 0, 1], 0));
    let socket = UdpSocket::bind(any_local_port).unwrap();

    println!("Socket address: {}", socket.local_addr().unwrap());

    {
        // Another handle to the same OS socket. It reads and writies to the same socket.
        let _cloned_socket = socket.try_clone().unwrap();
    }

    println!(
        "Is broadcast: {}, Time To Live: {}",
        socket.broadcast().unwrap(),
        socket.ttl().unwrap()
    );
    socket.set_broadcast(true).unwrap();

    // Reading from socket can be configured to return immediately.
    // Then when there are no bytes, WouldBlock error is returned.
    socket.set_nonblocking(true).unwrap();
}

fn run_client_server() {
    let (server_ready_notify, server_ready_recv) = unbounded();
    let thread = std::thread::spawn(move || server::echo(server_ready_notify));

    // Wait until the server is ready to accept requests.
    server_ready_recv.recv().unwrap();

    // Execute a client.
    client::client();

    thread.join().unwrap();
}

fn main() {
    bind_interface();
    socket_api();
    println!();
    run_client_server();
}
