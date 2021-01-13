mod domain;

#[macro_use]
mod utils;

pub use crate::domain::*;
pub use atomic_register_public::*;
pub use register_client_public::*;
pub use sectors_manager_public::*;
pub use stable_storage_public::*;
pub use transfer_public::*;

pub async fn run_register_process(config: Configuration) {
    unimplemented!()
}

pub mod atomic_register_public {
    use crate::{
        ClientRegisterCommand, OperationComplete, RegisterClient, SectorsManager, StableStorage,
        SystemRegisterCommand,
    };
    use std::sync::Arc;

    #[async_trait::async_trait]
    pub trait AtomicRegister {
        /// Send client command to the register. After it is completed, we expect
        /// callback to be called. Note that completion of client command happens after
        /// delivery of multiple system commands to the register, as the algorithm specifies.
        async fn client_command(
            &mut self,
            cmd: ClientRegisterCommand,
            operation_complete: Box<dyn FnOnce(OperationComplete) + Send + Sync>,
        );

        /// Send system command to the register.
        async fn system_command(&mut self, cmd: SystemRegisterCommand);
    }

    /// Idents are numbered starting at 1 (up to the number of processes in the system).
    /// Storage for atomic register algorithm data is separated into StableStorage.
    /// Communication with other processes of the system is to be done by register_client.
    /// And sectors must be stored in the sectors_manager instance.
    pub async fn build_atomic_register(
        self_ident: u8,
        metadata: Box<dyn StableStorage>,
        register_client: Arc<dyn RegisterClient>,
        sectors_manager: Arc<dyn SectorsManager>,
        processes_count: usize,
    ) -> (Box<dyn AtomicRegister>, Option<ClientRegisterCommand>) {
        unimplemented!()
    }
}

pub mod sectors_manager_public {
    use std::sync::Arc;
    use crate::{SectorIdx, SectorVec};
    use std::path::PathBuf;
    use std::io::SeekFrom;
    use std::path::Path;
    use tokio::task;
    use tokio::fs::File;
    use tokio::fs::OpenOptions;
    use tokio::prelude::*;
    // use std::thread::JoinHandle;
    // use tokio::runtime::{Builder, Runtime};
    use serde::{Deserialize, Serialize};
    use bincode;

    // use tokio::prelude::Future;


    
    #[async_trait::async_trait]
    pub trait SectorsManager: Send + Sync {
        /// Returns 4096 bytes of sector data by index.
        async fn read_data(&self, idx: SectorIdx) -> SectorVec;

        /// Returns timestamp and write rank of the process which has saved this data.
        /// Timestamps and ranks are relevant for atomic register algorithm, and are described
        /// there.
        async fn read_metadata(&self, idx: SectorIdx) -> (u64, u8);

        /// Writes a new data, along with timestamp and write rank to some sector.
        async fn write(&self, idx: SectorIdx, sector: &(SectorVec, u64, u8));
    }

    /// Path parameter points to a directory to which this method has exclusive access.
    pub fn build_sectors_manager(path: PathBuf) -> Arc<dyn SectorsManager> {
        // unimplemented!()
        // let task = tokio::fs::create_dir(super::META_DIR);
            // .and_then(|mut file| file.poll_write(b"hello, world!"))
            // .map(|res| {
            //     println!("{:?}", res);
            // }).map_err(|err| eprintln!("IO error: {:?}", err));

        // tokio::run(task);
        // let mut path_copy = path.clone();
        
        // let task = task::spawn( async move {
        //     path_copy.push(META_DIR);
        //     log::error!("path: {:?}", path_copy);
        //     tokio::fs::create_dir(path_copy.clone()).await.unwrap();
        //     path_copy.pop();
        //     path_copy.push(SECTORS_DIR);
        //     log::error!("path2: {:?}", path_copy);
        //     tokio::fs::create_dir(path_copy).await.unwrap();
            // tokio::join!(
            //     tokio::fs::create_dir(META_DIR),
            //     tokio::fs::create_dir(SECTORS_DIR),
            // );
        // });

        

        // tokio::join!(task);
        // let single_thread_runtime = Builder::new_current_thread().build().unwrap();
        // let multi_threaded_runtime = Runtime::new().unwrap();

    // Moving values into async task.
        // multi_threaded_runtime.block_on(task).unwrap();
        // log::error!("joined!");

        Arc::new(BasicSectorsManager{path})
    }

    pub struct BasicSectorsManager {
        path: PathBuf,
    }

    impl BasicSectorsManager {
        fn filepath(&self, idx: SectorIdx ) -> PathBuf {
            let mut tmp = self.path.clone();
            tmp.push(idx.to_string());
            tmp
        }
    }
    
    #[async_trait::async_trait]
    impl SectorsManager for BasicSectorsManager {

        async fn read_metadata(&self, idx: SectorIdx) -> (u64, u8) {
            let path = self.filepath(idx);
            let file = tokio::fs::File::open(&path).await;
            
            let mut res = vec![];

            match file {
                Ok(mut file) => {
                    file.seek(SeekFrom::Start(4096)).await.unwrap();
                    file.read_to_end(&mut res).await.unwrap();
                },
                Err(_) => {},
            }

            bincode::deserialize(&res).unwrap()
        }

        async fn read_data(&self, idx: SectorIdx) -> SectorVec {
            let path = self.filepath(idx);
            let file = tokio::fs::File::open(&path).await;
            
            let mut tmp : [u8; 4096] = [0; 4096];
            match file {
                Ok(mut file) => {
                    file.read_exact(&mut tmp).await.unwrap();
                },
                Err(_) => {},
            }

            let mut res = Vec::new();
            res.extend_from_slice(&tmp);
            SectorVec(res)
        }

        async fn write(&self, idx: SectorIdx, sector: &(SectorVec, u64, u8)) {
            let path = self.filepath(idx);
            let file = OpenOptions::new()
                .write(true)
                .open(&path)
                .await;
            
            let mut file = match file {
                Ok(file) => {
                    file
                },
                Err(_) => {
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(&path)
                        .await.unwrap()
                },
            };
            file.seek(SeekFrom::Start(0)).await.unwrap();
            let SectorVec(data) = &sector.0;
            file.write_all(&data).await.unwrap();
            let meta = (sector.1, sector.2);
            file.write_all(&bincode::serialize(&meta).unwrap()).await.unwrap();
        }
    }
}


const MAGIC: &[u8; 4] = &[0x61, 0x74, 0x64, 0x64];
const MSG_OFFSET : usize = 7;
// TODO those should be used sometime
// const RESPONSE_MSG_TYPE_ADD : u8 = 0x40;
// const MSG_READ : u8 = 0x1;
// const MSG_WRITE : u8 = 0x2;

const REQ_NUM_OFFSET : usize = 8;
const SECTOR_IDX_OFFSET : usize = 16;
const HDR_SIZE : usize = 24;
const BLOCK_SIZE : usize = 4096;

/// Your internal representation of RegisterCommand for ser/de can be anything you want,
/// we just would like some hooks into your solution to asses where the problem is, should
/// there be a problem.
pub mod transfer_public {
    use crate::domain::SectorVec;
    use crate::RegisterCommand;
    use crate::MAGIC;
    use crate::ClientRegisterCommand;
    use crate::ClientCommandHeader;
    use crate::ClientRegisterCommandContent;
    use std::convert::TryInto;
    use std::io::{Error, Read, Write, BufWriter, Cursor};


    #[derive(Debug, PartialEq)]
    pub enum Direction {
        Request,
        ReadResponse(SectorVec),
        WriteResponse,
    }


    pub fn deserialize_register_command(data: &mut dyn Read) -> Result<RegisterCommand, Error> {
        deserialize_register_command_generic(data, Direction::Request{})
    }

    fn deserialize_register_command_generic(
        data: &mut dyn Read, 
        direction : Direction) -> Result<RegisterCommand, Error> {
        let mut read_buf = vec![];
        let num = data.read_to_end(&mut read_buf).unwrap();

        if &read_buf[0..crate::MAGIC.len()] != crate::MAGIC {
            return safe_err_return!(format!("wrong Magic number: expected {:?}, got {:?}", crate::MAGIC, &read_buf[0..crate::MAGIC.len()]));
        }

        // response not supported for now
        assert_eq!(direction, Direction::Request{});

        if direction == Direction::Request {
            let content = if (read_buf[crate::MSG_OFFSET] & 0x1) != 0 {
                if num != crate::HDR_SIZE {
                    return safe_err_return!(format!("mismatched message size, expected {}, got {}", crate::HDR_SIZE, num))
                }
                ClientRegisterCommandContent::Read{}
            } else {
                if num != crate::HDR_SIZE + crate::BLOCK_SIZE {
                    return safe_err_return!(format!("mismatched message size, expected {}, got {}", crate::HDR_SIZE + crate::BLOCK_SIZE, num))
                }
                ClientRegisterCommandContent::Write{data: SectorVec((
                    &read_buf[crate::HDR_SIZE..]).to_vec()
                )}
            };

            let header = ClientCommandHeader {
                request_identifier: u64::from_be_bytes(read_buf[crate::REQ_NUM_OFFSET..crate::REQ_NUM_OFFSET+8].try_into().expect("internal error")),
                sector_idx: u64::from_be_bytes(read_buf[crate::SECTOR_IDX_OFFSET..crate::SECTOR_IDX_OFFSET+8].try_into().expect("internal error")) as crate::domain::SectorIdx,
            };

            Ok(RegisterCommand::Client(ClientRegisterCommand{
                header,
                content,
            }))
        } else {
            unimplemented!()
        }
    }


    fn serialize_register_command_generic(
        cmd: &RegisterCommand,
        writer: &mut dyn Write,
        direction: Direction,
    ) -> Result<(), Error> {
        let stream = writer;  // replace it with File or Cursor::new(Vec::<u8>::new()) for testing purposes
        let mut buf_writer = BufWriter::new(stream);
        match cmd {
            RegisterCommand::Client(ClientRegisterCommand{header, content}) => {
                let ClientCommandHeader{request_identifier, sector_idx} = header;
                let mut separable_part = BufWriter::new(Cursor::new(Vec::<u8>::new()));

                let msg_type : u8;

                match content {
                    ClientRegisterCommandContent::Read => {
                        msg_type = if let Direction::Request = direction {0x1} else {0x41};

                        if let Direction::Request = direction {
                            safe_unwrap!(separable_part.write_all(&sector_idx.to_be_bytes()));
                        }
                        if let Direction::ReadResponse(data) = direction {
                            let SectorVec(data) = data;
                            if data.len() != 4096 {
                                log::error!("malformed SectorVec! data length should be 4096, not {}", data.len());
                            }
                            safe_unwrap!(separable_part.write_all(&data));
                        }
                    },
                    ClientRegisterCommandContent::Write{data} => {
                        msg_type = if let Direction::Request = direction {0x2} else {0x42};

                        if let Direction::Request = direction {
                            safe_unwrap!(separable_part.write_all(&sector_idx.to_be_bytes()));
                            let SectorVec(data) = data;
                            if data.len() != 4096 {
                                log::error!("malformed SectorVec! data length should be 4096, not {}", data.len());
                            }
                            safe_unwrap!(separable_part.write_all(&data));
                        }
                    }
                }
                
                // common part
                vec![
                    buf_writer.write_all(MAGIC),
                    buf_writer.write_all(&[0x0, 0x0, 0x0]), // padding
                    buf_writer.write_all(&msg_type.to_be_bytes()),
                    buf_writer.write_all(&request_identifier.to_be_bytes()),
                ].into_iter().for_each(|x| {safe_unwrap!(x)});

                // part that depends on msg. type
                safe_unwrap!(buf_writer.write_all(separable_part.buffer()));
            },
            RegisterCommand::System(_) => {
                unimplemented!()
            },
        }
        Ok(())
    }

    pub fn serialize_register_command(
        cmd: &RegisterCommand,
        writer: &mut dyn Write,
    ) -> Result<(), Error> {
        serialize_register_command_generic(cmd, writer, Direction::Request{})
    }
}

pub mod register_client_public {
    use crate::SystemRegisterCommand;
    use std::sync::Arc;

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
}

pub mod stable_storage_public {
    #[async_trait::async_trait]
    /// A helper trait for small amount of durable metadata needed by the register algorithm
    /// itself. Again, it is only for AtomicRegister definition. StableStorage in unit tests
    /// is durable, as one could expect.
    pub trait StableStorage: Send + Sync {
        async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String>;

        async fn get(&self, key: &str) -> Option<Vec<u8>>;
    }
}
