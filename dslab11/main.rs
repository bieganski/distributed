use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use actix::clock::Duration;
use actix::{Actor, Addr, Recipient};
use log::LevelFilter;
use uuid::Uuid;

use crate::solution::{ProcessConfig, ProcessState, Raft, RaftMessage, StableStorage};

mod public_test;
mod solution;

#[actix_rt::main]
async fn main() {
    // Your solution may do some logging, so progress will be visible.
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let sender = ActixSender::default();

    // In real implementations timeouts have to be randomized.
    let (raft_process0, id0) =
        build_process(Duration::from_millis(500), 2, Box::new(sender.clone()));
    let (raft_process1, id1) =
        build_process(Duration::from_millis(1000), 2, Box::new(sender.clone()));
    sender.insert(id0, raft_process0.recipient());
    sender.insert(id1, raft_process1.recipient());
    actix::clock::delay_for(Duration::from_millis(2000)).await;
    // `remove` makes it possible to simulate network partitions.
    sender.remove(&id0);
}

fn build_process(
    election_timeout: Duration,
    processes_count: usize,
    sender: Box<dyn crate::solution::Sender>,
) -> (Addr<Raft>, Uuid) {
    let self_id = Uuid::new_v4();
    let config = ProcessConfig {
        self_id,
        election_timeout,
        processes_count,
    };

    (
        Raft::new(config, Box::new(RamStorage::default()), sender).start(),
        self_id,
    )
}

#[derive(Clone, Default)]
struct ActixSender {
    processes: Arc<Mutex<HashMap<Uuid, Recipient<RaftMessage>>>>,
}

impl ActixSender {
    fn insert(&self, id: Uuid, addr: Recipient<RaftMessage>) {
        self.processes.lock().unwrap().insert(id, addr);
    }

    fn remove(&self, id: &Uuid) {
        self.processes.lock().unwrap().remove(id);
    }
}

impl crate::solution::Sender for ActixSender {
    fn send(&self, target: &Uuid, msg: RaftMessage) {
        if let Some(addr) = self.processes.lock().unwrap().get(target) {
            let addr = addr.clone();
            actix::spawn(async move {
                let _ = addr.send(msg).await.unwrap();
            });
        }
    }

    fn broadcast(&self, msg: RaftMessage) {
        let map = self.processes.lock().unwrap();
        for addr in map.values() {
            let addr = addr.clone();
            actix::spawn(async move {
                let _ = addr.send(msg).await;
            });
        }
    }
}

#[derive(Default, Clone)]
struct RamStorage {
    state: Arc<Mutex<Option<ProcessState>>>,
}

impl StableStorage for RamStorage {
    fn put(&mut self, state: &ProcessState) {
        *self.state.lock().unwrap().deref_mut() = Some(*state);
    }

    fn get(&self) -> Option<ProcessState> {
        *self.state.lock().unwrap().deref()
    }
}
