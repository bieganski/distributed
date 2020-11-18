mod client;
mod server;

use crossbeam_channel::unbounded;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn tcp_stream_api(stream: &mut TcpStream) {
    stream.set_nodelay(true).expect("set_nodelay call failed");
    println!("TTL: {}", stream.ttl().unwrap());
    println!("Write timeout: {:#?}", stream.write_timeout().unwrap());

    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();

    {
        // Independent reference to the same underlying socket.
        let _cloned = stream.try_clone().unwrap();
    }
}

fn tcp_listener() {
    // A tcp socket is created by binding it to a local port.
    let listener = TcpListener::bind("127.0.0.1:8888").unwrap();

    // `incoming` returns an iterator.
    for _stream in listener.incoming().take(0) {
        // Process TcpStream...
    }

    // Alternatively, you can call `accept` in a loop.
    while false {
        match listener.accept() {
            Ok((mut socket, addr)) => {
                println!("new client: {:?}", addr);
                tcp_stream_api(&mut socket);
            }
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }

    println!("Local address: {:#?}", listener.local_addr().unwrap());
    {
        // TCP socket handle can be cloned.
        let _clone = listener.try_clone().unwrap();
    }

    println!("Time To Live: {}", listener.ttl().unwrap());

    // Reading from socket can be configured to return immediately.
    // Then when there are no bytes, WouldBlock error is returned.
    listener.set_nonblocking(true).unwrap();
}

fn run_client_server() {
    let (server_ready_notify, server_ready_recv) = unbounded();
    let thread = std::thread::spawn(move || server::echo_once(server_ready_notify));
    // Wait until the server is ready to accept requests.
    server_ready_recv.recv().unwrap();

    // Execute a client.
    client::send_msg();

    thread.join().unwrap();
}

fn main() {
    tcp_listener();
    run_client_server();
}
