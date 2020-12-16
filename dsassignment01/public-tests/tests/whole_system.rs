use assignment_1_solution::{
    setup_system, Configuration, ModuleRef, PlainSender, PlainSenderMessage,
    ReliableBroadcastModule, System, SystemMessage, SystemMessageContent, SystemMessageHeader,
};
use core::panic;
use crossbeam_channel::{unbounded, Receiver, Sender};
use ntest::timeout;
use rand::Rng;
use std::time::Duration;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tests_lib::RamStorage;
use uuid::Uuid;

#[derive(Clone)]
struct FaultySender {
    tx: Sender<(Uuid, PlainSenderMessage)>,
}

impl PlainSender for FaultySender {
    fn send_to(&self, uuid: &Uuid, msg: PlainSenderMessage) {
        self.tx.send((*uuid, msg)).unwrap();
    }
}

struct SystemSetup {
    #[allow(dead_code)]
    system: System,
    retransmission_delay: Duration,
    #[allow(dead_code)]
    sender: FaultySender,
    processes: HashSet<Uuid>,
    rx: Receiver<(Uuid, PlainSenderMessage)>,
    modules: HashMap<Uuid, ModuleRef<ReliableBroadcastModule>>,
    delivered: HashMap<Uuid, Arc<Mutex<HashMap<SystemMessageHeader, SystemMessageContent>>>>,
    initiated_broadcasts: Vec<(Uuid, Vec<u8>)>,
}

impl SystemSetup {
    fn new(broadcasts_num: usize, retransmission_delay: Duration) -> Self {
        let mut processes = HashSet::new();
        while processes.len() < broadcasts_num {
            processes.insert(Uuid::new_v4());
        }
        let mut system = System::new();
        let (tx, rx) = unbounded();
        let sender = FaultySender { tx };
        let mut modules = HashMap::new();
        let mut delivered = HashMap::new();
        for id in &processes {
            let curr_mod_delivered = Arc::new(Mutex::new(HashMap::new()));
            delivered.insert(*id, curr_mod_delivered.clone());
            let lurb = setup_system(
                &mut system,
                Configuration {
                    self_process_identifier: *id,
                    processes: processes.clone(),
                    stable_storage: Box::new(RamStorage::new()),
                    sender: Box::new(sender.clone()),
                    retransmission_delay,
                    delivered_callback: Box::new(move |SystemMessage { header, data }| {
                        match curr_mod_delivered.lock().unwrap().insert(header, data) {
                            None => {}
                            Some(_) => {
                                panic!("Module delivered the same message for a second time");
                            }
                        }
                    }),
                },
            );
            modules.insert(*id, lurb);
        }
        Self {
            system,
            sender,
            retransmission_delay,
            processes,
            rx,
            modules,
            delivered,
            initiated_broadcasts: Vec::new(),
        }
    }

    fn start_random_broadcasts(&mut self, min_send_broadcasts: usize, max_send_broadcasts: usize) {
        let mut rng = rand::thread_rng();
        for id in &self.processes {
            for _ in 0..rng.gen_range(min_send_broadcasts, max_send_broadcasts + 1) {
                let mut vec = Vec::with_capacity(rng.gen_range(0, 5));
                for _ in 0..vec.capacity() {
                    vec.push(rng.gen_range(0u8, 255u8));
                }
                self.initiated_broadcasts.push((*id, vec.clone()));
                // Start broadcast
                self.modules
                    .get(id)
                    .unwrap()
                    .send(SystemMessageContent { msg: vec });
            }
        }
        self.initiated_broadcasts.sort_unstable();
    }

    fn forward_messages_until_silence(&mut self, delivering_probability: f64) {
        let silence_period_length =
            Duration::from_secs_f64(self.retransmission_delay.as_secs_f64() * 1.5);
        let mut rng = rand::thread_rng();
        loop {
            match self.rx.recv_timeout(silence_period_length) {
                Ok((id, msg)) => {
                    if rng.gen_bool(delivering_probability) {
                        if let Some(module) = self.modules.get(&id) {
                            match msg {
                                PlainSenderMessage::Acknowledge(ack) => {
                                    // println!("Ack from {:?} of {:?}@{:?}", ack.proc, ack.hdr.message_id, ack.hdr.message_source_id);
                                    module.send(ack);
                                }
                                PlainSenderMessage::Broadcast(bct) => {
                                    // println!("Broadcast from {:?} of {:?}@{:?}", bct.forwarder_id, bct.message.header.message_id, bct.message.header.message_source_id);
                                    module.send(bct);
                                }
                            }
                        }
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    return;
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    panic!("unexpected error");
                }
            }
        }
    }

    fn check_delivered_broadcasts(&self) {
        for id in &self.processes {
            let mut vec = Vec::new();
            for (hdr, content) in self.delivered.get(id).unwrap().lock().unwrap().iter() {
                vec.push((hdr.message_source_id, content.msg.clone()));
            }
            vec.sort_unstable();
            assert_eq!(self.initiated_broadcasts, vec);
        }
    }
}

#[test]
#[timeout(1000)]
fn communication_stops_if_everything_works() {
    let mut sys_setup = SystemSetup::new(3, Duration::from_millis(50));
    sys_setup.start_random_broadcasts(0, 3);
    sys_setup.forward_messages_until_silence(1f64);
    sys_setup.check_delivered_broadcasts();
}

#[test]
#[timeout(5000)]
fn communication_stops_with_faulty_links() {
    let mut sys_setup = SystemSetup::new(3, Duration::from_millis(50));
    sys_setup.start_random_broadcasts(0, 3);
    sys_setup.forward_messages_until_silence(0.5f64);
    sys_setup.check_delivered_broadcasts();
}
