use crate::raft::utils::{build_process, ActixSender, RamStorage};
use crate::raft::ClientRequest;
use crate::solution::{DistributedSet, SetOperation};
use actix::clock::Duration;
use actix::Actor;
use log::LevelFilter;
use std::ops::Deref;
use uuid::Uuid;

mod public_test;
mod raft;
mod solution;

#[actix_rt::main]
async fn main() {
    // This example will show how Raft implementation can be used. You only care about
    // ClientRequest message and committed_node_X_rx, which is a channel which
    // gets a stream of LogEntries in order.
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let sender = ActixSender::default();
    let storage0 = RamStorage::default();

    let ident0 = Uuid::new_v4();
    let ident1 = Uuid::new_v4();
    let processes = vec![ident0, ident1];

    // We expect that raft0 will win leadership. Setup is a little bit shaky, but
    // designing a solid Raft interface in Rust is far too much for this lab.
    let (raft0, committed_node_0_rx) = build_process(
        Duration::from_millis(200),
        &processes,
        ident0,
        Box::new(sender.clone()),
        Box::new(storage0.clone()),
    );
    let (raft1, committed_node_1_rx) = build_process(
        Duration::from_millis(500),
        &processes,
        ident1,
        Box::new(sender.clone()),
        Box::new(RamStorage::default()),
    );
    sender.insert(ident0, raft0.clone().recipient());
    sender.insert(ident1, raft1.recipient());

    actix::clock::delay_for(Duration::from_millis(1000)).await;
    // We expect leader to be elected by now.

    // We decided our implementation will store Vec<u8> in the log for data. We do not care
    // what is inside.
    raft0
        .send(ClientRequest("Hello, Raft!".to_string().into_bytes()))
        .await
        .unwrap();
    // Real Raft implementation would have a response mechanism to inform that request
    // was processed. We have to cut corners.
    actix::clock::delay_for(Duration::from_millis(1000)).await;
    assert_eq!(
        b"Hello, Raft!",
        committed_node_1_rx.try_recv().unwrap().data.deref()
    );

    let set = DistributedSet::new(raft0, committed_node_0_rx).start();

    set.send(SetOperation::Add(0)).await.unwrap();

    // This silences warnings.
    sender.remove(&ident0);
    sender.remove(&ident1);
}
