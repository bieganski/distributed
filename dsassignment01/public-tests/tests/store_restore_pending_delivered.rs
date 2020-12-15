use ntest::timeout;
use std::collections::HashSet;
use std::iter::FromIterator;

use std::path::PathBuf;
use assignment_1_solution::{
    build_stubborn_broadcast, SystemBroadcastMessage, SystemMessage, SystemMessageContent,
    SystemMessageHeader, BasicStableStorage, StorageType
};
use tests_lib::TestSender;
use uuid::Uuid;

#[test]
#[timeout(300)]
fn store_retriever_test() {
    let mut path = PathBuf::new();
    path.push("/home/mateusz/matinek");
    let mut a = BasicStableStorage{root: path, free_delivered_id: 0, free_pending_id: 0};

    // testowane - działa
    // let name = BasicStableStorage::get_key_name(StorageType::DELIVERED, &Uuid::new_v4(), 2137);
    let hdr_msg = SystemMessageHeader{message_id: Uuid::new_v4(), message_source_id: Uuid::new_v4()};
    let content = SystemMessageContent{msg: vec![0x66, 0x67, 0x68]};
    a.store_pending(&hdr_msg, &content);
    let vec = a.restore_pending(&vec![hdr_msg.message_source_id]);
    assert_eq!(vec[0].header, hdr_msg);
    assert_eq!(vec.len(), 1);
    println!("{:?}", vec);

    a.store_delivered(&hdr_msg);
    let vec = a.restore_delivered(&vec![hdr_msg.message_source_id]);
    assert_eq!(vec[0], hdr_msg);
    assert_eq!(vec.len(), 1);
}
