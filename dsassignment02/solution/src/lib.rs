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


use tokio::net::TcpListener;
use std::io::{Cursor};
use std::sync::{Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};


// Hmac uses also sha2 crate.
use hmac::{Hmac, Mac, NewMac};
use sha2::Sha256;

use log;

static HMAC_TAG_SIZE: usize = 32;


pub async fn run_register_process(config: Configuration) {
    let my_addr = &config.public.tcp_locations[config.public.self_rank as usize - 1];
    let listener = TcpListener::bind(my_addr)
        .await
        .unwrap();

    let (mut register, pending_cmd) = build_atomic_register(
        1,
        Box::new(BasicStableStorage::new(config.public.storage_dir.clone()).await),
        Arc::new(BasicRegisterClient::new(config.public.tcp_locations.clone())),
        build_sectors_manager(config.public.storage_dir),
        1,
    )
    .await;

    let mut header = [0_u8; 8];
    let read_content : &mut Vec<u8> = &mut vec![0_u8; 16]; // req. nr. + sector idx + cmd content + HMAC
    let write_content : &mut Vec<u8> = &mut vec![0_u8; 16 + 4096];
    let hmac_signature : &mut Vec<u8> = &mut vec![0_u8; HMAC_TAG_SIZE];

    // aaaa
    // [208, 113, 246, 189, 248, 159, 121, 131, 174, 124, 11, 29, 85, 232, 232, 192, 251, 145, 248, 98, 22, 173, 198, 87, 250, 129, 106, 138, 60, 63, 144, 192]

    // 5
    // [208, 44, 139, 113, 235, 130, 112, 216, 61, 51, 205, 224, 101, 27, 220, 123, 244, 158, 170, 150, 137, 97, 254, 144, 2, 103, 175, 53, 114, 29, 36, 174]

    loop {
        let (mut stream, _) = listener.accept().await.unwrap();

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

        let content : &mut Vec<u8> = if header[7] == 0x1 {read_content} else {write_content};
        stream
            .read_exact(content)
            .await
            .expect("Less data then expected");
        stream
            .read_exact(hmac_signature)
            .await
            .expect("Less data then expected");

        let mut command = std::io::Read::chain(&header as &[u8], &content as &[u8]);

        match deserialize_register_command_send(&mut command) {
            Ok(reg_cmd) => {
                log::info!("[run_register_process] message parsed successfully");
                match reg_cmd {
                    RegisterCommand::Client(cmd) => {
                        let mut status_code = StatusCode::Ok;

                        let mut mac = Hmac::<Sha256>::new_varkey(&config.hmac_client_key).expect("HMAC can take key of any size");
                        mac.update(&header);
                        mac.update(&content);
                        let proper_mac = mac.finalize().into_bytes();
                        if hmac_signature.as_slice() != &*proper_mac {
                            log::error!("wrong HMAC signature! \ngot: {:?}\nexptected: {:?}", hmac_signature.as_slice(), &*proper_mac);
                            status_code = StatusCode::AuthFailure;
                        }
                
                        if cmd.header.sector_idx >= config.public.max_sector {
                            log::error!("[run_register_process] Too high sector_idx requested! Max is {}, Requested: {}", cmd.header.sector_idx, config.public.max_sector);
                            status_code = StatusCode::InvalidSectorIndex;
                        }

                        if status_code != StatusCode::Ok {
                            // too high idx or wrong HMAC 
                            let op_return : OperationReturn = match cmd.content {
                                ClientRegisterCommandContent::Read => OperationReturn::Read(ReadReturn{read_data: None}),
                                ClientRegisterCommandContent::Write {data: _} => OperationReturn::Write{},
                            };
                            send_response_to_client(stream, cmd.clone(), status_code, op_return, &config.hmac_client_key).await;
                            continue;
                        }

                        let hmac_client_key = config.hmac_client_key.clone();
                        register.client_command(cmd.clone(), Box::new(move |op_complete: domain::OperationComplete| {
                            match op_complete.status_code {
                                status_code@StatusCode::Ok => {
                                    tokio::spawn(async move {
                                        send_response_to_client(
                                            stream,
                                            cmd,
                                            status_code,
                                            op_complete.op_return,
                                            &hmac_client_key,
                                            ).await;
                                    });
                                
                                },
                                code => panic!("internal error: error status code {:?} should be handled before", code),
                            }
                        })).await;
                    },
                    RegisterCommand::System(cmd) => {
                        register.system_command(cmd).await;
                    },
                }
            },
            Err(_) => {
                log::error!("[run_register_process] message parsing failed!");
                continue;
            }
        }
    } // loop
}

fn serialize_status_code(status_code: StatusCode) -> u8 {
    let res = match status_code {
        StatusCode::Ok => {0x0},
        StatusCode::AuthFailure{} => {0x1},
        StatusCode::InvalidSectorIndex{} => {0x2},
    };
    res as u8
}

async fn send_response_to_client(
    mut stream: tokio::net::TcpStream, 
    cmd: ClientRegisterCommand,
    status_code: StatusCode,
    op_return: OperationReturn,
    hmac_client_key: &[u8; 32],
    ) {
        let status_code : u8 = serialize_status_code(status_code);

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

        stream.write_all(serialized_response.get_ref()).await.unwrap();
}
