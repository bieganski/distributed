use assignment_1_solution::{build_reliable_broadcast, PlainSender, PlainSenderMessage, StubbornBroadcast, StubbornBroadcastModule, System, SystemAcknowledgmentMessage, SystemBroadcastMessage, SystemMessageContent, SystemMessageHeader, ReliableBroadcast, StableStorage, SystemMessage, build_stable_storage,};
use crossbeam_channel::{unbounded, Sender, Receiver};
use ntest::timeout;
use tests_lib::RamStorage;
use uuid::Uuid;
use tempfile::{tempdir};

fn create_broadcast(id: Uuid) -> (System, Box<dyn ReliableBroadcast>, Receiver<Uuid>) {
    create_broadcast_storage(id, Box::new(RamStorage::new()))
}

fn create_broadcast_storage(id: Uuid, storage: Box<dyn StableStorage>)
    -> (System, Box<dyn ReliableBroadcast>, Receiver<Uuid>) {
    create_broadcast_storage_callback(id, storage, Box::new(|_| {}))
}

fn create_broadcast_storage_callback(id: Uuid, storage: Box<dyn StableStorage>, callback: Box<dyn Fn(SystemMessage) + Send>)
                            -> (System, Box<dyn ReliableBroadcast>, Receiver<Uuid>) {
    // given
    let (tx, rx) = unbounded();
    let mut system = System::new();
    let sbeb = system.register_module(StubbornBroadcastModule::new(Box::new(
        TestStubbornBroadcast { tx },
    )));
    (
        system,
        build_reliable_broadcast(sbeb, storage, id, 1, callback),
        rx
    )
}

#[test]
#[timeout(300)]
fn reliable_broadcast_uses_stubborn_broadcast() {
    //given
    let id = Uuid::new_v4();
    let (_system, mut reliable_broadcast, rx) = create_broadcast(id);

    // when
    reliable_broadcast.broadcast(SystemMessageContent { msg: vec![] });

    // then
    assert_eq!(rx.recv().unwrap(), id);
}

#[test]
#[timeout(300)]
fn reliable_broadcast_restoring_delivered() {
    //given
    let id = Uuid::new_v4();
    let root_storage_dir = tempdir().unwrap();

    // in this case callback gets called
    let storage = build_stable_storage(root_storage_dir.path().to_path_buf());
    let callback = Box::new(|_| {});
    check_delivered(id, storage, callback);

    // in this case callback does not get called, because message is in delivered
    let storage1 = build_stable_storage(root_storage_dir.path().to_path_buf());
    let callback1 = Box::new(|_| { assert!(false) });
    let rx = check_delivered(id, storage1, callback1);
    // message sent right after restoring
    rx.recv().unwrap();
}

fn check_delivered(id :Uuid, storage: Box<dyn StableStorage>,
                   callback: Box<dyn Fn(SystemMessage) + Send>) -> Receiver<Uuid> {
    let msg = SystemBroadcastMessage {
        forwarder_id: id,
        message: SystemMessage {
            header: SystemMessageHeader {
                message_source_id: id,
                message_id: id,
            },
            data: SystemMessageContent { msg: (0..100).collect() }
        }
    };

    let (_system, mut reliable_broadcast, rx) =
        create_broadcast_storage_callback(id, storage, callback);
    reliable_broadcast.deliver_message(msg);
    rx
}


use log::{LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode};

#[test]
#[timeout(300)]
fn reliable_broadcast_restoring_pending() {
    //given
    TermLogger::init(LevelFilter::Trace, Config::default(), TerminalMode::Mixed)
    .expect("No interactive terminal");

    let id = Uuid::new_v4();
    let root_storage_dir = tempdir().unwrap();

    let storage = build_stable_storage(root_storage_dir.path().to_path_buf());
    let rx = check_pending(id, storage);
    // new message has been forwarded
    assert!(rx.recv().is_ok());

    let storage1 = build_stable_storage(root_storage_dir.path().to_path_buf());
    let rx1 = check_pending(id, storage1);
    // message sent right after restoring
    rx1.recv().unwrap();
    // message was not forwarded, because it was already in pending
    assert!(rx1.recv().is_err());
}

fn check_pending(id :Uuid, storage: Box<dyn StableStorage>) -> Receiver<Uuid> {
    let msg = SystemBroadcastMessage {
        forwarder_id: id,
        message: SystemMessage {
            header: SystemMessageHeader {
                message_source_id: id,
                message_id: id
            },
            data: SystemMessageContent { msg: (0..100).collect() }
        }
    };

    let (_system, mut reliable_broadcast, rx) = create_broadcast_storage(id, storage);
    reliable_broadcast.deliver_message(msg.clone());
    rx
}

struct TestStubbornBroadcast {
    tx: Sender<Uuid>,
}

impl StubbornBroadcast for TestStubbornBroadcast {
    fn broadcast(&mut self, msg: SystemBroadcastMessage) {
        self.tx.send(msg.forwarder_id).unwrap();
    }

    fn receive_acknowledgment(&mut self, _proc: Uuid, _msg: SystemMessageHeader) {}

    fn send_acknowledgment(&mut self, _proc: Uuid, _msg: SystemAcknowledgmentMessage) {}

    fn tick(&mut self) {}
}

struct NothingSender {}

impl PlainSender for NothingSender {
    fn send_to(&self, _uuid: &Uuid, _msg: PlainSenderMessage) {}
}
