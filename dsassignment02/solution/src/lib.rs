#[macro_use]
mod utils;

mod storage;
mod domain;
mod atomic_register;
mod register_client;
mod sectors_manager;
mod transfer;

pub use crate::domain::*;
pub use crate::storage::*;
pub use crate::transfer::*;
pub use crate::atomic_register::atomic_register_public::*;
pub use crate::register_client::register_client_public::*;
pub use crate::sectors_manager::sectors_manager_public::*;
pub use crate::transfer::transfer_public::*;

pub use transfer_public::*;
pub use stable_storage_public::*;

#[allow (unused_imports)]
use std::time::Duration;

use tokio::net::TcpListener;
use std::io::{Cursor};
use std::sync::{Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::collections::HashMap;
use tokio::sync::Mutex;

// Hmac uses also sha2 crate.
use hmac::{Hmac, Mac, NewMac};
use sha2::Sha256;
use log;
use tokio::sync::mpsc::{unbounded_channel};


static HMAC_TAG_SIZE: usize = 32;

use lazy_static::lazy_static;

// aaaa
// [208, 113, 246, 189, 248, 159, 121, 131, 174, 124, 11, 29, 85, 232, 232, 192, 251, 145, 248, 98, 22, 173, 198, 87, 250, 129, 106, 138, 60, 63, 144, 192]

// 5
// [208, 44, 139, 113, 235, 130, 112, 216, 61, 51, 205, 224, 101, 27, 220, 123, 244, 158, 170, 150, 137, 97, 254, 144, 2, 103, 175, 53, 114, 29, 36, 174]



lazy_static! {
    pub static ref SENT_BCAST:  std::sync::Mutex<HashMap<u8, Vec< SystemRegisterCommand>>>       = std::sync::Mutex::new(HashMap::new());
    pub static ref SENT_SINGLE: std::sync::Mutex<HashMap<u8, Vec<(SystemRegisterCommand, u8)>>>  = std::sync::Mutex::new(HashMap::new()); // (cmd, target)
}

pub struct TestRegisterClient {
    rank: u8,
    register_client: Box<dyn RegisterClient>,
}

impl TestRegisterClient {
    fn new(register_client: Box<dyn RegisterClient>, rank: u8) -> Self {
        SENT_SINGLE.lock().unwrap().insert(rank, Vec::new());
        SENT_BCAST.lock().unwrap().insert(rank, Vec::new());
        
        Self{
            rank,
            register_client,
        }
    }
}

pub fn summary() {

    let mut s = String::from("");
    s.push_str("\n\n=========== SUMMARY:");
    for (k, v) in SENT_BCAST.lock().unwrap().iter() {
        s.push_str(&format!("\n@@@@@ BROADCASTED BY {}:\n", k));
        for el in v.iter() {
            s.push_str(&format!("{}", el));
        }
    }
    for (k, v) in SENT_SINGLE.lock().unwrap().iter() {
        s.push_str(&format!("\n@@@@@ DIRECTLY SENT BY {}:\n", k));
        for (cmd, target) in v.iter() {
            s.push_str(&format!("targeted to {}\n{}", target, cmd));
        }
    }
    log::info!("{}", s);
}

#[async_trait::async_trait]
impl RegisterClient for TestRegisterClient {
    async fn send(&self, msg: Send) {
        (*std::sync::Mutex::lock(&SENT_SINGLE).unwrap()).get_mut(&self.rank).unwrap().push(((*msg.cmd).clone(), msg.target as u8));
        self.register_client.send(msg).await
    }

    async fn broadcast(&self, msg: Broadcast) {
        SENT_BCAST.lock().unwrap().get_mut(&self.rank).unwrap().push((*msg.cmd).clone());
        self.register_client.broadcast(msg).await
    }
}


const NUM_WORKERS: usize = 2;

struct Ready{who: usize}

pub async fn run_register_process(config_save: Configuration) {

    let my_addr = &config_save.public.tcp_locations[config_save.public.self_rank as usize - 1];
    let listener = TcpListener::bind(my_addr)
        .await
        .unwrap();

    let register_client = BasicRegisterClient::new(config_save.public.tcp_locations.clone(), config_save.hmac_system_key);
    let register_client = Arc::new(TestRegisterClient::new(Box::new(register_client), config_save.public.self_rank));
    
    let (rdy_tx, rdy_rx) = unbounded_channel();
    let rdy_rx = Arc::new(Mutex::new(rdy_rx));
    let cmd_txs = Arc::new(Mutex::new(vec![]));
    let ret_rxs = Arc::new(Mutex::new(vec![]));

    let msg_owners : Arc<Mutex<HashMap<uuid::Uuid, usize>>> = Arc::new(Mutex::new(HashMap::new()));

    for idx in 0..NUM_WORKERS {
        
        let config = config_save.clone();
        
        let storage = BasicStableStorage::new(config.public.storage_dir.clone()).await;

        let (mut register, pending_cmd) = build_atomic_register_generic(
            config.public.self_rank,
            Box::new(storage.clone()),
            register_client.clone(),
            build_sectors_manager(config.public.storage_dir),
            config.public.tcp_locations.len(),
            msg_owners.clone(), 
            idx
        )
        .await;

        let (cmd_tx, mut cmd_rx) = unbounded_channel();
        cmd_txs.lock().await.push(cmd_tx.clone());

        let (ret_tx, ret_rx) = unbounded_channel();
        ret_rxs.lock().await.push(ret_rx);

        let rdy_tx = rdy_tx.clone();
        tokio::spawn(async move {
            log::info!("worker {} starts loop...", idx);
            let pending_cmd = match pending_cmd {
                None => None,
                Some(cmd) => match cmd.content {
                    ClientRegisterCommandContent::Read => None,
                    ClientRegisterCommandContent::Write{..} => Some(RegisterCommand::Client(cmd))
                }
            };
            // if there is no pending command from reboot, then I can change state to READY
            match pending_cmd {
                None => rdy_tx.send(Ready{who: idx}).unwrap_or_else(|_| {log::info!("BAD THINGS HAPPENED")}),
                Some(cmd) => cmd_tx.send(cmd).unwrap(),
            };
            loop {
                let cmd = cmd_rx.recv().await.unwrap();

                match cmd {
                    RegisterCommand::Client(cmd) => {
                        log::info!("Worker received Client command {}...", cmd.header.request_identifier);
                        storage.put(&format!("pending{}", idx), &bincode::serialize(&cmd).unwrap()).await.unwrap();
                        let ret_tx = ret_tx.clone();
                        let rdy_tx = rdy_tx.clone();
                        let storage = storage.clone();
                        register.client_command(cmd.clone(), Box::new(move |op_complete: domain::OperationComplete| {
                            log::info!("~~~~~~~~~~~ MATINEK - SKONCZYLEM ROBIC {} ~~~~~~~~~~", cmd.header.request_identifier);
                            tokio::spawn(async move {
                                storage.drop(&format!("pending{}", idx)).await.unwrap_or_else(|_| {});
                            });
                            ret_tx.send(op_complete.op_return).unwrap_or_else(|_| {log::info!("BAD THINGS HAPPENED")});
                            rdy_tx.send(Ready{who: idx}).unwrap_or_else(|_| {log::info!("BAD THINGS HAPPENED")});
                        })).await;
                    },
                    RegisterCommand::System(cmd) => {
                        log::info!("-------- REGISTER parsed!");
                        register.system_command(cmd).await;
                    },
                }
            }
        });
    }
    
    let act_register : Arc<std::sync::Mutex<usize>> = Arc::new(std::sync::Mutex::new(0));

    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        
        let config = config_save.clone();
        let hmac_client_key = config.hmac_client_key.clone();
        let rdy_rx = rdy_rx.clone();
        let cmd_txs = cmd_txs.clone();
        let ret_rxs = ret_rxs.clone();
        let act_register = act_register.clone();

        let msg_owners = msg_owners.clone();
        tokio::spawn(async move {
            let mut header = [0_u8; 8];
            let mut hmac_signature : &mut Vec<u8> = &mut vec![0_u8; HMAC_TAG_SIZE];
            let msg_owners = msg_owners.clone();
            let ret_rxs = ret_rxs.clone();
            // let (queue_tx, queue_rx) = unbounded_channel();
            loop {
                // let queue_res = queue_rx.try_recv(); // TODO tu jestem
                // match queue_res

                let num = stream.peek(&mut header).await.unwrap();

                if num == 0 {
                    log::info!("EXTREMELY IMPORTANT - MATINEK, REMOVE ME IF U SURE. connection closed.");
                    return ();
                }

                stream
                    .read_exact(&mut header)
                    .await
                    .expect("Less data then expected");
            
                if header[..4] != MAGIC_NUMBER {
                    log::error!("[run_register_process] wrong Magic Number!");
                    continue;
                }
        
                let supported_msg_types = 1..7;
        
                if ! supported_msg_types.contains(&header[7]) {
                    log::error!("[run_register_process] wrong message type! Got {:?}", header[7]);
                    continue
                }
        
                log::info!("[proc {}]: header[7]: {}", config.public.self_rank, header[7]);
        
                // let content : &mut Vec<u8> = if header[7] == 0x1 {read_content} else {write_content};
                let mut content : Vec<u8> = match header[7] {
                    0x1 => {vec![0_u8; 16]}, // client read
                    0x2 => {vec![0_u8; 16 + 4096]}, // client write
                    
                    0x3 => {vec![0_u8; 32 + 0]}, // system read, no content
                    0x4 => {vec![0_u8; 32 + 16 + 4096]}, // system value
                    0x5 => {vec![0_u8; 32 + 16 + 4096]}, // system write
                    0x6 => {vec![0_u8; 32 + 0]}, // system ack, no content
                    _ => {panic!("internal error: i should have been handled earlier..")},
                };
        
                stream
                    .read_exact(&mut content)
                    .await
                    .expect("Less data then expected");
        
                // TODO REMOVE ME
                for i in 0..content.len() - 3 {
                    if &MAGIC_NUMBER as &[u8] == &content[i..i+4 as usize] {
                        assert!(false);
                    }
                }
        
                stream
                    .read_exact(&mut hmac_signature)
                    .await
                    .expect("Less data then expected");

                let mut command = std::io::Read::chain(&header as &[u8], &content as &[u8]);
                let cmd = deserialize_register_command_send(&mut command).unwrap();
                let mut mac = match cmd {
                    RegisterCommand::Client(_) => {
                        Hmac::<Sha256>::new_varkey(&config.hmac_client_key).expect("HMAC can take key of any size")
                    },
                    RegisterCommand::System(_) => {
                        Hmac::<Sha256>::new_varkey(&config.hmac_system_key).expect("HMAC can take key of any size")
                    }
                };
                mac.update(&header);
                mac.update(&content);

                match cmd.clone() {
                    RegisterCommand::Client(cmd) => {
                        match cmd.content {
                            ClientRegisterCommandContent::Write{data} => {
                                log::error!(">>>>>>>>>>>>>>> WRITE: {}", cmd.header.request_identifier);
                            },
                            _ => {}
                        }
                    },
                    _ => {},
                };

                let proper_mac = mac.finalize().into_bytes();
                if hmac_signature.as_slice() != &*proper_mac {
                    log::error!("wrong HMAC signature! \ngot: {:?}\nexptected: {:?}", hmac_signature.as_slice(), &*proper_mac);
                    let status_code = StatusCode::AuthFailure;

                    match cmd {
                        RegisterCommand::Client(cmd) => {
                            let op_return : OperationReturn = match cmd.content {
                                ClientRegisterCommandContent::Read => OperationReturn::Read(ReadReturn{read_data: None}),
                                ClientRegisterCommandContent::Write {data: _} => OperationReturn::Write{},
                            };
                            send_response_to_client(&mut stream, cmd.clone(), status_code, op_return, &config.hmac_client_key).await;
                            continue;
                        },
                        RegisterCommand::System(_) => {
                            log::info!("received System Command with wrong HMAC tag, abandoning..");
                            continue;
                        },
                    }
                }

                match cmd.clone() {
                    RegisterCommand::Client(client_cmd) => {
                        if client_cmd.header.sector_idx >= config.public.max_sector {
                            log::warn!("[run_register_process] Too high sector_idx requested! Max is {}, Requested: {}", client_cmd.header.sector_idx, config.public.max_sector);
                            let status_code = StatusCode::InvalidSectorIndex;
                            let op_return : OperationReturn = match client_cmd.content {
                                ClientRegisterCommandContent::Read => OperationReturn::Read(ReadReturn{read_data: None}),
                                ClientRegisterCommandContent::Write {data: _} => OperationReturn::Write{},
                            };
                            send_response_to_client(&mut stream, client_cmd.clone(), status_code, op_return, &config.hmac_client_key).await;
                            continue;
                        }
                        
                        let Ready{who} = rdy_rx.lock().await.recv().await.unwrap();
                        
                        // let who;
                        // loop {
                        //     let Ready{who: whoo} = rdy_rx.lock().await.recv().await.unwrap();
                        //     log::info!("{} claims that is ready, maybe abandoning...", whoo);
                        //     if whoo == 0 {
                        //         who = whoo;
                        //         break;
                        //     }
                        // }

                        (*cmd_txs.lock().await).get_mut(who).unwrap().send(cmd).unwrap();

                        log::info!("sent client cmd for processing to atomic register {} ..", who);

                        let ret_rxs = &mut (*ret_rxs.lock().await);
                        let ret_rx = ret_rxs.get_mut(who).unwrap();
                        let op_return = ret_rx.recv().await.unwrap();

                        send_response_to_client(
                            &mut stream,
                            client_cmd,
                            StatusCode::Ok,
                            op_return,
                            &hmac_client_key,
                        ).await;
                    },
                    RegisterCommand::System(system_cmd) => {
                        log::info!("DOSTALEM SYSTEM MSG :)");
                        log::info!("looking for {:?} :)", &system_cmd.header.msg_ident);
                        let msg_owners = &(*msg_owners.lock().await);
                        for (k, v) in msg_owners.iter( ) {
                            log::info!("AA: {:?}, {:?}", k, v);
                        }
                        let who = match msg_owners.get(&system_cmd.header.msg_ident) {
                            Some(who) => *who,
                            None => {
                                let mut guard = act_register.lock().unwrap();
                                let val = *guard % NUM_WORKERS;
                                *guard += 1;
                                val
                            },
                        };
                        (*cmd_txs.lock().await).get_mut(who).unwrap().send(cmd).unwrap();

                    },
                };

            }; // loop
        });
    }
}

async fn send_response_to_client(
    stream: &mut tokio::net::TcpStream, 
    cmd: ClientRegisterCommand,
    status_code: StatusCode,
    op_return: OperationReturn,
    hmac_client_key: &[u8; 32],
    ) {
        let status_code = status_code as u8;

        let dir : Direction = match cmd.content {
            ClientRegisterCommandContent::Read => {
                if let OperationReturn::Read(ret) = op_return {
                    let read_data = ret.read_data; 
                    Direction::ReadResponse(read_data, status_code)
                } else {
                    // error in program logic
                    panic!("internal error, in statement: 'if let OperationReturn::Read(ret) = op_complete.op_return'");
                }
            },
            ClientRegisterCommandContent::Write{data: _} => {
                Direction::WriteResponse(status_code)
            },
        };
        let mut serialized_response = Cursor::new(vec![]);
        safe_unwrap!(serialize_register_command_generic(&RegisterCommand::Client(cmd), &mut serialized_response, dir));

        let mut mac = Hmac::<Sha256>::new_varkey(hmac_client_key).expect("HMAC can take key of any size");
        mac.update(serialized_response.get_ref());
        let response_mac = mac.finalize().into_bytes();
        std::io::Write::write_all(&mut serialized_response, &response_mac).unwrap();

        stream.write_all(serialized_response.get_ref()).await.unwrap_or_else(|e| {log::warn!("Cannot send response to client! Error message: {:?}", e)});
}
