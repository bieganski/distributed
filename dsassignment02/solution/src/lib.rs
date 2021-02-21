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
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::net::TcpStream;

static HMAC_TAG_SIZE: usize = 32;


const NUM_WORKERS: usize = 20;

struct Ready{who: usize}

pub async fn run_register_process(config_save: Configuration) {

    let my_addr = &config_save.public.tcp_locations[config_save.public.self_rank as usize - 1];
    let listener = TcpListener::bind(my_addr)
        .await
        .unwrap();

    let register_client = BasicRegisterClient::new(config_save.public.tcp_locations.clone(), config_save.hmac_system_key);
    let register_client = Arc::new(register_client);
    
    let (rdy_tx, rdy_rx) = unbounded_channel();
    let rdy_rx = Arc::new(Mutex::new(rdy_rx));
    let cmd_txs = Arc::new(Mutex::new(vec![]));

    let msg_owners : Arc<Mutex<HashMap<uuid::Uuid, usize>>> = Arc::new(Mutex::new(HashMap::new()));

    for idx in 0..NUM_WORKERS {
        
        let config = config_save.clone();
        let hmac_client_key = config.hmac_client_key.clone();
        
        let mut storage = BasicStableStorage::new(config.public.storage_dir.clone()).await;

        let (mut register, pending_cmd) = build_atomic_register_generic(
            config.public.self_rank,
            Box::new(storage.clone()),
            register_client.clone(),
            build_sectors_manager(config.public.storage_dir),
            config.public.tcp_locations.len(),
            msg_owners.clone(), 
            idx,
        )
        .await;

        let (cmd_tx, mut cmd_rx) : (UnboundedSender<(RegisterCommand, Option<Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>>)>, _) = unbounded_channel();
        cmd_txs.lock().await.push(cmd_tx.clone());

        let rdy_tx = rdy_tx.clone();
        tokio::spawn(async move {
            let pending_cmd = match pending_cmd {
                None => None,
                Some(cmd) => match cmd.content {
                    ClientRegisterCommandContent::Read => None,
                    ClientRegisterCommandContent::Write{..} => Some(RegisterCommand::Client(cmd))
                }
            };
            // if there is no pending command from reboot, then I can change state to READY
            match pending_cmd {
                None => rdy_tx.send(Ready{who: idx}).unwrap_or_else(|_| {log::info!("Error sending Ready message...")}),
                Some(cmd) => {
                    cmd_tx.send((cmd, None)).unwrap()
                },
            };
            loop {
                let (cmd, arc) = cmd_rx.recv().await.unwrap();

                match cmd {
                    RegisterCommand::Client(cmd) => {
                        storage.put(&format!("pending{}", idx), &bincode::serialize(&cmd).unwrap()).await.unwrap();
                        let rdy_tx = rdy_tx.clone();
                        let storage = storage.clone();
                        register.client_command(cmd.clone(), Box::new(move |op_complete: domain::OperationComplete| {
                            let op_return = op_complete.op_return.clone();
                            tokio::spawn(async move {
                                storage.drop(&format!("pending{}", idx)).await.unwrap_or_else(|_| {});

                                match arc {
                                    Some(arc) => send_response_to_client(
                                        &mut *arc.lock().await,
                                        cmd,
                                        StatusCode::Ok,
                                        op_return,
                                        &hmac_client_key,
                                    ).await,
                                    None => {},
                                }
                            });
                            rdy_tx.send(Ready{who: idx}).unwrap_or_else(|_| {log::info!("Error receiving Ready message...")});
                        })).await;
                    },
                    RegisterCommand::System(cmd) => {
                        register.system_command(cmd).await;
                    },
                }
            }
        });
    }
    
    let act_register : Arc<std::sync::Mutex<usize>> = Arc::new(std::sync::Mutex::new(0));

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let (mut read_stream, write_stream) = stream.into_split();

        let arc = Arc::new(Mutex::new(write_stream));
        
        let config = config_save.clone();
        let rdy_rx = rdy_rx.clone();
        let cmd_txs = cmd_txs.clone();
        let act_register = act_register.clone();

        let msg_owners = msg_owners.clone();
        tokio::spawn(async move {
            let mut header = [0_u8; 8];
            let mut hmac_signature : &mut Vec<u8> = &mut vec![0_u8; HMAC_TAG_SIZE];
            let msg_owners = msg_owners.clone();

            loop {
                let num = read_stream.peek(&mut header).await.unwrap();

                if num == 0 {
                    return ();
                }

                read_stream
                    .read_exact(&mut header)
                    .await
                    .expect("Less data then expected");
            
                if header[..4] != MAGIC_NUMBER {
                    log::error!("[run_register_process] wrong Magic Number, consume 8 bytes and read next..");
                    continue;
                }
        
                let supported_msg_types = 1..7;
        
                if ! supported_msg_types.contains(&header[7]) {
                    log::error!("[run_register_process] wrong message type! Got {:?}", header[7]);
                    continue
                }
        
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
        
                read_stream
                    .read_exact(&mut content)
                    .await
                    .expect("Less data then expected");
        
                read_stream
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
                            send_response_to_client(&mut *arc.lock().await, cmd.clone(), status_code, op_return, &config.hmac_client_key).await;
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
                            send_response_to_client(&mut *arc.lock().await, client_cmd, status_code, op_return, &config.hmac_client_key).await;
                            continue;
                        }
                        
                        let Ready{who} = rdy_rx.lock().await.recv().await.unwrap();

                        (*cmd_txs.lock().await).get_mut(who).unwrap().send((cmd, Some(arc.clone()))).unwrap();
                    },
                    RegisterCommand::System(system_cmd) => {
                        let msg_owners = &(*msg_owners.lock().await);
                        let who = match msg_owners.get(&system_cmd.header.msg_ident) {
                            Some(who) => *who,
                            None => {
                                let mut guard = act_register.lock().unwrap();
                                let val = *guard % NUM_WORKERS;
                                *guard += 1;
                                val
                            },
                        };
                        (*cmd_txs.lock().await).get_mut(who).unwrap().send((cmd, None)).unwrap();
                    },
                };

            }; // loop
        });
    }
}

async fn send_response_to_client(
    stream: &mut tokio::net::tcp::OwnedWriteHalf, 
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
