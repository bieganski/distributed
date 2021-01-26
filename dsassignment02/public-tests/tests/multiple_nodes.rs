use assignment_2_solution::{
    RegisterClient, Send, Broadcast, SystemRegisterCommand, Configuration, PublicConfiguration, run_register_process, 
    RegisterCommand, ClientRegisterCommand, ClientRegisterCommandContent, serialize_register_command, SectorVec, ClientCommandHeader,
    summary, SENT_BCAST, SENT_SINGLE, MAGIC_NUMBER
};
use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use ntest::timeout;
use lazy_static::lazy_static;
use tempfile::tempdir;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use hmac::{Hmac, Mac, NewMac};
use sha2::Sha256;
use tokio::task::JoinHandle;
use std::convert::TryInto;

// TODO tu jestem
// * dodać sector manager (na razie 1)
// * przetestować + stworzyć test corectness
// * dodać crash recovery
// * zrobić test crash recovery
// * zrobić ostatnie poprawki
// * dodać więcej sector managerów


#[tokio::test]
#[timeout(4000)]
async fn multiple_nodes() {
    let _ = env_logger::builder().is_test(true).try_init();

    let range = 1..3;

    let mut addrs = Vec::<(String, u16)>::new();
    let mut tcp_port = 10_000;
    let hmac_client_key = [0x61; 32]; // =0x61 == 'a'

    for _ in range.clone() {
        addrs.push(("127.0.0.1".to_string(), tcp_port));
        tcp_port += 1;
    }

    let mut handles = vec![];

    for i in range {
        assert!(i > 0);
        let config = Configuration {
            public: PublicConfiguration {
                tcp_locations: addrs.clone(),
                self_rank: i,
                max_sector: 20,
                storage_dir: tempdir().unwrap().into_path(),
            },
            hmac_system_key: [1; 64],
            hmac_client_key,
        };
        println!("spawn");
        handles.push(tokio::spawn(run_register_process(config)));
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    let request_identifier = 1778;

    let write_cmd = RegisterCommand::Client(ClientRegisterCommand {
        header: ClientCommandHeader {
            request_identifier,
            sector_idx: 12,
        },
        content: ClientRegisterCommandContent::Write {
            data: SectorVec(vec![3; 4096]),
        },
    });

    let mut stream = TcpStream::connect(addrs[0].clone())
        .await
        .expect("Could not connect to TCP port");

    // when
    send_cmd(&write_cmd, &mut stream, &hmac_client_key).await;


    const EXPECTED_RESPONSES_SIZE: usize = 48;
    let mut buf = [0_u8; EXPECTED_RESPONSES_SIZE];
    stream
        .read_exact(&mut buf)
        .await
        .expect("Less data then expected");

    // asserts for write response
    assert_eq!(&buf[0..4], MAGIC_NUMBER.as_ref());
    assert_eq!(buf[7], 0x42);
    assert_eq!(buf[6], 0x0); // response status - OK
    assert_eq!(
        u64::from_be_bytes(buf[8..16].try_into().unwrap()),
        request_identifier
    );
    assert!(hmac_tag_is_ok(&hmac_client_key, &buf));

    // tokio::time::sleep(Duration::from_millis(500)).await;
    
    summary();
    
    // for h in handles.drain(..) {
    //     tokio::join!(h);
    // }

    let source_rank = 1;
    let target_rank = 2;

    {
        let bcast = SENT_BCAST.lock().unwrap();
        let bcasted_from_source : &Vec<SystemRegisterCommand> = bcast.get(&source_rank).unwrap();
        assert_eq!(bcasted_from_source.len(), 2); // read and write

        let bcasted_from_target : &Vec<SystemRegisterCommand> = bcast.get(&target_rank).unwrap();
        assert_eq!(bcasted_from_target.len(), 0);
    }

    {
        let direct = SENT_SINGLE.lock().unwrap();
        let direct_from_source = direct.get(&source_rank).unwrap();
        assert_eq!(direct_from_source.len(), 2); // value and ack
        // assert!(direct_from_source.iter().filter(|x| { (let SystemRegisterCommandContent::Value{} = x.content) as bool; }))
        // let bcasted_from_target : &Vec<SystemRegisterCommand> = bcast.get(&target_rank).unwrap();

        let direct_from_target = direct.get(&target_rank).unwrap();
        assert_eq!(direct_from_target.len(), 2); // value and ack
    }    
}

type HmacSha256 = Hmac<Sha256>;

async fn send_cmd(register_cmd: &RegisterCommand, stream: &mut TcpStream, hmac_client_key: &[u8]) {
    let mut data = Vec::new();
    serialize_register_command(register_cmd, &mut data).unwrap();
    let mut key = HmacSha256::new_varkey(hmac_client_key).unwrap();
    key.update(&data);
    data.extend(key.finalize_reset().into_bytes());

    stream.write_all(&data).await.unwrap();
}

fn hmac_tag_is_ok(key: &[u8], data: &[u8]) -> bool {
    let boundary = data.len() - 32; // - HMAC
    let mut mac = HmacSha256::new_varkey(key).unwrap();
    mac.update(&data[..boundary]);
    mac.verify(&data[boundary..]).is_ok()
}
