pub mod register_client_public {
    use crate::transfer_public::serialize_register_command;
    use crate::SystemRegisterCommand;
    use std::sync::Arc;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use crate::domain::RegisterCommand;
    use std::time::Duration;

    #[async_trait::async_trait]
    /// We do not need any public implementation of this trait. It is there for use
    /// in AtomicRegister. In our opinion it is a safe bet to say some structure of
    /// this kind must appear in your solution.
    pub trait RegisterClient: core::marker::Send + core::marker::Sync {
        /// Sends a system message to a single process.
        async fn send(&self, msg: Send);

        /// Broadcasts a system message to all processes in the system, including self.
        async fn broadcast(&self, msg: Broadcast);
    }

    pub struct Broadcast {
        pub cmd: Arc<SystemRegisterCommand>,
    }

    pub struct Send {
        pub cmd: Arc<SystemRegisterCommand>,
        /// Identifier of the target process. Those start at 1.
        pub target: usize,
    }

    pub struct BasicRegisterClient {
        tcp_locations: Vec<(String, u16)>,
    }

    impl BasicRegisterClient {
        pub fn new(tcp_locations: Vec<(String, u16)>,) -> Self {
            BasicRegisterClient{
                tcp_locations,
            }
        }
    }

    // #[async_trait::async_trait]
    impl BasicRegisterClient {
        #[allow(dead_code)]
        const TIMEOUT : Duration = Duration::from_millis(500);
        
        async fn serialize(cmd : Arc<SystemRegisterCommand>) ->Vec<u8> {
            let mut serialized_msg = vec![];
            safe_unwrap!(
                serialize_register_command(
                    &RegisterCommand::System(
                        (*cmd).clone()), 
                    &mut serialized_msg)
            );
            serialized_msg
        }
    }

    #[async_trait::async_trait]
    impl RegisterClient for BasicRegisterClient {

        async fn send(&self, msg: Send) {
            let serialized_msg = Self::serialize(msg.cmd).await;

            let addr = self.tcp_locations[msg.target - 1].clone();
            let mut stream = TcpStream::connect(addr.clone())
                .await
                .expect(&format!("cannot connect to TCP of target {} (addr: {:?}", msg.target, addr));
            
            stream.write_all(&serialized_msg).await.unwrap();
        }

        async fn broadcast(&self, msg: Broadcast) {
            let serialized_msg = Self::serialize(msg.cmd).await;

            for addr in self.tcp_locations.iter() {
                let mut stream = TcpStream::connect(addr.clone())
                    .await
                    .expect(&format!("cannot connect to TCP of (addr: {:?}", addr));
            
                stream.write_all(&serialized_msg).await.unwrap();
            }
        }
    }
}
