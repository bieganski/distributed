mod domain;

#[macro_use]
mod utils;

pub use crate::domain::*;
pub use atomic_register_public::*;
pub use register_client_public::*;
pub use sectors_manager_public::*;
pub use stable_storage_public::*;
pub use transfer_public::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;


// Hmac uses also sha2 crate.
use hmac::{Hmac, Mac, NewMac};
use sha2::Sha256;

use log;

static HMAC_TAG_SIZE: usize = 32;


pub async fn run_register_process(config: Configuration) {
    // struct PublicConfiguration {
    //     /// Storage for durable data.
    //          storage_dir: PathBuf,
    //     /// Host and port, indexed by identifiers, of every other process.
    //          tcp_locations: Vec<(String, u16)>,
    //     /// Identifier of this process. Identifiers start at 1.
    //          self_rank: u8,
    //     /// First NOT supported sector index.
    //          max_sector: u64,
    // }
    let my_addr = &config.public.tcp_locations[config.public.self_rank as usize - 1];
    let listener = TcpListener::bind(my_addr)
        .await
        .unwrap();

    let (mut stream, _) = listener.accept().await.unwrap();

    let mut header = [0_u8; 8];
    let mut read_content : &mut Vec<u8> = &mut vec![0_u8; 16]; // req. nr. + sector idx + cmd content + HMAC
    let mut write_content : &mut Vec<u8> = &mut vec![0_u8; 16 + 4096];
    let mut hmac_signature : &mut Vec<u8> = &mut vec![0_u8; HMAC_TAG_SIZE];

    // aaaa
    // [208, 113, 246, 189, 248, 159, 121, 131, 174, 124, 11, 29, 85, 232, 232, 192, 251, 145, 248, 98, 22, 173, 198, 87, 250, 129, 106, 138, 60, 63, 144, 192]

    // 5
    // [208, 44, 139, 113, 235, 130, 112, 216, 61, 51, 205, 224, 101, 27, 220, 123, 244, 158, 170, 150, 137, 97, 254, 144, 2, 103, 175, 53, 114, 29, 36, 174]

    loop {
        let mut mac = Hmac::<Sha256>::new_varkey(&config.hmac_client_key).expect("HMAC can take key of any size");
        stream
            .read_exact(&mut header)
            .await
            .expect("Less data then expected");
    
        if header[..4] != MAGIC_NUMBER {
            log::error!("[run_register_process] wrong Magic Number!");
            continue;
        }

        if header[7] != 0x1 && header[7] != 0x2 {
            log::error!("[run_register_process] wrong message type! Got {:?}", header[7]);
                continue
        }

        let buf : &mut Vec<u8> = if header[7] == 0x1 {read_content} else {write_content};
        stream
            .read_exact(buf)
            .await
            .expect("Less data then expected");

        mac.update(&header);
        mac.update(&buf);

        stream
            .read_exact(hmac_signature)
            .await
            .expect("Less data then expected");

        let mut buf = Read::chain(&header as &[u8], &buf as &[u8]); // .chain(&hmac_signature as &[u8]);

        let proper_mac = mac.finalize().into_bytes();

        if hmac_signature.as_slice() != &*proper_mac {
            log::error!("wrong HMAC signature! \ngot: {:?}\nexptected: {:?}", hmac_signature.as_slice(), &*proper_mac);
        }

        match deserialize_register_command(&mut buf) {
            Ok(reg_cmd) => {
                log::info!("[run_register_process] message parsed successfully");

            },
            Err(_) => {
                log::error!("[run_register_process] message parsing failed!");
                continue;
            }
        }

    } // loop
    unimplemented!()
}

pub mod atomic_register_public {
    use crate::domain::SectorVec;
    use std::collections::HashSet;
    use std::collections::HashMap;

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
        // TODO that None below handling
        let state = SystemNodeState{
            ts: Ts(0),
            wr: Rank(self_ident),
            val: SectorVec(Vec::new()),
            writeval: SectorVec(Vec::new()),
            readval: SectorVec(Vec::new()),
            rid: RequestId(0),
            readlist: HashMap::<Rank, (Ts, Rank, SectorVec)>::new(),
            acklist: HashSet::<Rank>::new(),
            reading: false,
            writing: false,
            operation_complete: None,
        };
        (Box::new(BasicAtomicRegister{self_id: Rank(self_ident), state, metadata, register_client, sectors_manager, processes_count}), None)
    }

    // timestamp
    #[derive(Debug, Clone, Copy)]
    struct Ts(u64);

    // id of read operation
    #[derive(Copy, Clone, Debug)]
    struct RequestId(u64);

    // process (and 'wr', at most 'rank')
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct Rank(u8);

    // impl std::ops::Add<u64> for RequestId {
    //     type Output = RequestId;
    
    //     fn add(self, rhs: u64) -> RequestId {
    //         RequestId(self.0 + rhs)
    //     }
    // }

    impl std::ops::AddAssign<u64> for RequestId {
        fn add_assign(&mut self, rhs: u64) {
            self.0 += rhs;
        }
    }

    struct SystemNodeState {
        ts: Ts,
        wr: Rank,
        writeval: SectorVec,
        readval: SectorVec,
        val: SectorVec,
        rid: RequestId,
        readlist: HashMap<Rank, (Ts, Rank, SectorVec)>,
        acklist: HashSet<Rank>,
        reading: bool,
        writing: bool,
        operation_complete: Option<Box<dyn FnOnce(OperationComplete) + Send + Sync>>,
    }

    struct BasicAtomicRegister {
        metadata: Box<dyn StableStorage>,
        register_client: Arc<dyn RegisterClient>,
        sectors_manager: Arc<dyn SectorsManager>,
        processes_count: usize,
        state: SystemNodeState,
        self_id: Rank,
    }

    use crate::ClientRegisterCommandContent;
    use crate::SystemCommandHeader;
    use crate::SystemRegisterCommandContent;
    use crate::OperationReturn;
    use crate::StatusCode;
    use crate::ReadReturn;
    use log;

    impl BasicAtomicRegister {
        fn highest(&self) -> Rank {

            let mut max = (Ts(0), Rank(0));
            let mut max_id = Rank(0);
            for (key, (ts, wr, _)) in &self.state.readlist {
                if (ts.0, wr.0) > ((max.0).0, (max.1).0) {
                    max_id = Rank(key.0);
                    max = (*ts, *wr)
                }
            }
            log::info!("[highest]: found max for {}", max_id.0);
            assert_ne!(0, max_id.0);
            max_id
        }
    }

    #[async_trait::async_trait]
    impl AtomicRegister for BasicAtomicRegister {
        async fn client_command(
            &mut self,
            cmd: ClientRegisterCommand,
            operation_complete: Box<dyn FnOnce(OperationComplete) + Send + Sync>) {
                self.state.operation_complete = Some(operation_complete);
                match cmd.content {
                    ClientRegisterCommandContent::Read => {
                        log::info!("[client_command] captured Client Read (from {})", self.self_id.0);
                        // rid := rid + 1;
                        // store(rid);
                        // readlist := [ _ ] `of length` N;
                        // acklist := [ _ ] `of length` N;
                        // reading := TRUE;
                        // trigger < sbeb, Broadcast | [READ_PROC, rid] >;
                        self.state.rid += 1; // TOOD maybe += 'processes_count'?
                        safe_unwrap!(self.metadata.put("rid", &self.state.rid.0.to_ne_bytes()).await);
                        self.state.readlist.drain();
                        self.state.acklist.drain();
                        self.state.reading = true;
                        assert_eq!(self.state.writing, false);
                        log::info!("[client_command][client-system] sending broadcast ReadProc (from {})", self.self_id.0);
                        self.register_client.broadcast(crate::Broadcast{
                            cmd: Arc::new(SystemRegisterCommand{
                                 header: SystemCommandHeader{
                                     process_identifier: self.self_id.0,
                                     msg_ident: uuid::Uuid::new_v4(),
                                     read_ident: self.state.rid.0,
                                     sector_idx: cmd.header.sector_idx, 
                                 },
                                 content: SystemRegisterCommandContent::ReadProc{}
                            })
                        }).await;
                    },
                    ClientRegisterCommandContent::Write{data} => {
                        log::info!("[client_command] captured Client Write (from {})", self.self_id.0);
                        // rid := rid + 1;
                        // writeval := v;
                        // acklist := [ _ ] `of length` N;
                        // readlist := [ _ ] `of length` N;
                        // writing := TRUE;
                        // store(wr, ts, rid, writeval, writing);
                        // trigger < sbeb, Broadcast | [READ_PROC, rid] >;
                        self.state.rid += 1;
                        self.state.writeval = data;
                        self.state.readlist.drain();
                        self.state.acklist.drain();
                        self.state.writing = true;
                        assert_eq!(self.state.reading, false);
                        vec![
                            self.metadata.put("wr",  &self.state.wr.0.to_ne_bytes()).await,
                            self.metadata.put("ts", &self.state.ts.0.to_ne_bytes()).await,
                            self.metadata.put("rid", &self.state.rid.0.to_ne_bytes()).await,
                            self.metadata.put("writeval", &self.state.writeval.0).await,
                            self.metadata.put("writing", &bincode::serialize(&self.state.writing).unwrap()).await,
                        ].into_iter().for_each(|x| {safe_unwrap!(x)});

                        log::info!("[client_command][client-system] sending broadcast ReadProc (from {})", self.self_id.0);
                        self.register_client.broadcast(crate::Broadcast{
                            cmd: Arc::new(SystemRegisterCommand{
                                 header: SystemCommandHeader{
                                    process_identifier: self.self_id.0,
                                    msg_ident: uuid::Uuid::new_v4(),
                                    read_ident: self.state.rid.0,
                                    sector_idx: cmd.header.sector_idx, 
                                 },
                                 content: SystemRegisterCommandContent::ReadProc{}
                            })
                        }).await;
                    },
                }
            }

        async fn system_command(&mut self, cmd: SystemRegisterCommand) {
            let response_header =  SystemCommandHeader {
                process_identifier: self.self_id.0,
                ..cmd.header
            };

            // TODO tu jestem
            // * brak obsługi wielu sektorów - co ze zmienną val?
            // * store val, writeval
            match cmd.content {
                SystemRegisterCommandContent::ReadProc => {
                    log::info!("[{}][system_command] captured ReadProc from {}", self.self_id.0, cmd.header.process_identifier);
                    // trigger < pl, Send | p, [VALUE, r, ts, wr, val] >;
                    self.register_client.send(crate::Send{
                        target: cmd.header.process_identifier as usize, // TODO u8 cast
                        cmd: Arc::new(SystemRegisterCommand {
                            header: response_header.clone(),
                            content: SystemRegisterCommandContent::Value {
                                timestamp: self.state.ts.0,
                                write_rank: self.state.wr.0,
                                sector_data: self.state.val.clone(),
                            },
                        })
                    }).await;
                },
                SystemRegisterCommandContent::Value{timestamp, write_rank, sector_data} => {
                    log::info!("[{}][system_command] captured Value from {}", self.self_id.0, cmd.header.process_identifier);
                    if cmd.header.read_ident != self.state.rid.0 {
                        return ();
                    }
                    assert_eq!(!false && false, false); // TODO operator priority check
                    if !self.state.reading && !self.state.writing {
                        log::error!("TODO REMOVE ME: test error!: got Value when not performed any action");
                        return ();
                    }

                    let ts = Ts(timestamp);
                    let wr = Rank(write_rank);
                    self.state.readlist.insert(Rank(cmd.header.process_identifier), (ts, wr, sector_data));

                    if (self.state.readlist.len() <= self.processes_count / 2) {
                        return ();
                    }

                    let (maxts, rr, readval) = self.state.readlist.remove(&(self.highest())).unwrap();
                    self.state.readval = readval;
                    self.state.readlist.drain();
                    self.state.acklist.drain();

                    // upon event <sl, Deliver | q, [VALUE, r, ts', wr', v'] > such that r == rid do
                    // readlist[q] := (ts', wr', v');
                    // if #(readlist) > N / 2 and (reading or writing) then
                        // (maxts, rr, readval) := highest(readlist);
                        // readlist := [ _ ] `of length` N;
                        // acklist := [ _ ] `of length` N;
                        // if reading = TRUE then
                        //     trigger < sbeb, Broadcast | [WRITE_PROC, rid, maxts, rr, readval] >;
                        // else
                        //     trigger < sbeb, Broadcast | [WRITE_PROC, rid, maxts + 1, rank(self), writeval] >;


                    if self.state.reading {
                        self.register_client.broadcast(crate::Broadcast{
                            cmd: Arc::new(SystemRegisterCommand{
                                    header: response_header.clone(),
                                    content: SystemRegisterCommandContent::WriteProc{
                                        timestamp: maxts.0,
                                        write_rank: rr.0,
                                        data_to_write: self.state.readval.clone(),
                                    }
                            })
                        }).await;
                    } else {
                        self.register_client.broadcast(crate::Broadcast{
                            cmd: Arc::new(SystemRegisterCommand{
                                    header: response_header.clone(),
                                    content: SystemRegisterCommandContent::WriteProc{
                                        timestamp: maxts.0 + 1,
                                        write_rank: self.self_id.0,
                                        data_to_write: self.state.writeval.clone(),
                                    }
                            })
                        }).await;
                    }
                },
                SystemRegisterCommandContent::WriteProc{timestamp, write_rank, data_to_write} => {
                    log::info!("[{}][system_command] captured WriteProc from {}", self.self_id.0, cmd.header.process_identifier);
                    // upon event < sbeb, Deliver | p, [WRITE_PROC, r, ts', wr', v'] > do
                    // if (ts', wr') > (ts, wr) then
                    //     (ts, wr, val) := (ts', wr', v');
                    //     store(ts, wr, val);
                    // trigger < pl, Send | p, [ACK, r] >;

                    if (timestamp, write_rank) > (self.state.ts.0, self.state.wr.0) {
                        self.state.ts = Ts(timestamp);
                        self.state.wr = Rank(write_rank);
                        self.state.val = data_to_write;

                        vec![
                            self.metadata.put("wr",  &self.state.wr.0.to_ne_bytes()).await,
                            self.metadata.put("ts", &self.state.ts.0.to_ne_bytes()).await,
                            self.metadata.put("val", &self.state.val.0).await,
                        ].into_iter().for_each(|x| {safe_unwrap!(x)});
                    }
                    self.register_client.send(crate::Send{
                        target: cmd.header.process_identifier as usize, // TODO u8 cast
                        cmd: Arc::new(SystemRegisterCommand {
                            header: response_header.clone(),
                            content: SystemRegisterCommandContent::Ack{},
                        })
                    }).await;
                },
                SystemRegisterCommandContent::Ack => {
                    // upon event < pl, Deliver | q, [ACK, r] > such that r == rid do
                    // acklist[q] := Ack;
                    // if #(acklist) > N / 2 and (reading or writing) then
                    //     acklist := [ _ ] `of length` N;
                    //     if reading = TRUE then
                    //         reading := FALSE;
                    //         trigger < nnar, ReadReturn | readval >;
                    //     else
                    //         writing := FALSE;
                    //         store(writing);
                    //         trigger < nnar, WriteReturn >;
                    log::info!("[{}][system_command] captured Ack from {}", self.self_id.0, cmd.header.process_identifier);
                    if cmd.header.read_ident != self.state.rid.0 {
                        return ();
                    }
                    self.state.acklist.insert(Rank(cmd.header.process_identifier));
                    if self.state.reading && self.state.writing {
                        log::error!("internal error: reading and writing simulataneously!");
                    }
                    if (self.state.acklist.len() <= self.processes_count / 2) || !(self.state.reading || self.state.writing) {
                        return ();
                    }
                    self.state.acklist.drain();

                    let op_return : crate::domain::OperationReturn;
                    if self.state.reading {
                        self.state.reading = false;
                        op_return = OperationReturn::Read(ReadReturn{read_data: Some(self.state.readval.clone())});
                    } else {
                        // writing
                        self.state.writing = false;
                        op_return = OperationReturn::Write;
                    }

                    // that's bad I know
                    let op_complete = None;
                    let op_complete = std::mem::replace(&mut self.state.operation_complete, op_complete);
                    
                    (op_complete.unwrap())(OperationComplete{
                        status_code: StatusCode::Ok,
                        request_identifier: self.state.rid.0,
                        op_return,
                    });
                },
            }
        }
    }
}

pub mod sectors_manager_public {
    use std::sync::Arc;
    use crate::{SectorIdx, SectorVec};
    use std::path::PathBuf;
    use std::io::SeekFrom;
    use tokio::fs::OpenOptions;
    use tokio::prelude::*;
    use bincode;
    
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
                    bincode::deserialize(&res).unwrap()
                },
                Err(_) => {
                    (0_u64, 0_u8)
                },
            }
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
