#[cfg(test)]
pub(crate) mod tests {
    use crate::keys::{ROOT_CERT, SERVER_FULL_CHAIN, SERVER_PRIVATE_KEY};
    use crate::solution::{SecureClient, SecureServer};
    use crate::MacKeyProviderType;
    use crossbeam_channel::{unbounded, Sender};
    use ntest::timeout;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::ops::Deref;
    use std::sync::{Arc, Barrier};

    #[test]
    #[timeout(300)]
    fn secure_link_correctly_transmits_messages() {
        // given
        let msgs = vec![
            "".to_string(),
            "hello".to_string(),
            "a bit longer message to transmit".to_string(),
        ];
        let msgs_clone = msgs.clone();
        let (tx, _) = unbounded();

        run_communication(
            tx,
            Box::new(move |mut client| {
                // when
                for msg in msgs_clone {
                    client.send_msg(msg.as_bytes().to_vec());
                }
            }),
            &mut |mut server| {
                // then
                for msg in &msgs {
                    assert_eq!(msg.as_bytes(), server.recv_message().unwrap().deref())
                }
            },
        );
    }

    // Below are test utils which need to be in `public_test.rs`

    pub(crate) struct RawDataObserver {
        stream: TcpStream,
        tx: Sender<u8>,
    }

    impl Read for RawDataObserver {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.stream.read(buf)
        }
    }

    impl Write for RawDataObserver {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            for byte in buf {
                let _ = self.tx.send(*byte);
            }
            self.stream.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.stream.flush()
        }
    }

    /// This code makes sure that nor client nor server will finish - and drop
    /// TCP connection - before the other one
    pub(crate) fn run_communication(
        tx: Sender<u8>,
        client_code: Box<dyn FnOnce(SecureClient<RawDataObserver>) + Send>,
        server_code: &mut dyn FnMut(SecureServer<TcpStream>),
    ) {
        let (client, server) = setup_tcp_based_secure_communication(tx);

        let barrier = Arc::new(Barrier::new(2));
        let barrier_c = barrier.clone();

        let client = std::thread::spawn(move || {
            client_code(client);

            barrier_c.wait();
        });

        server_code(server);

        barrier.wait();
        assert!(matches!(client.join(), Ok(())));
    }

    pub(crate) fn setup_tcp_based_secure_communication(
        tx: Sender<u8>,
    ) -> (SecureClient<RawDataObserver>, SecureServer<TcpStream>) {
        let mac_key_provider = MacKeyProviderType::new(vec![80, 81]);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        println!("Server listening on: {}", listener.local_addr().unwrap());
        let client =
            TcpStream::connect(("127.0.0.1", listener.local_addr().unwrap().port())).unwrap();
        let client_data_observer = RawDataObserver { stream: client, tx };
        let server = listener.incoming().next().unwrap().unwrap();

        (
            SecureClient::new(client_data_observer, &mac_key_provider, ROOT_CERT),
            SecureServer::new(
                server,
                &mac_key_provider,
                SERVER_PRIVATE_KEY,
                SERVER_FULL_CHAIN,
            ),
        )
    }
}
