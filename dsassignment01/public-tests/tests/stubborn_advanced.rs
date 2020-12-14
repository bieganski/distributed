use ntest::timeout;
use std::collections::HashSet;
use std::iter::FromIterator;

use assignment_1_solution::{
    build_stubborn_broadcast, SystemBroadcastMessage, SystemMessage, SystemMessageContent,
    SystemMessageHeader,
    SystemAcknowledgmentMessage
};
use tests_lib::TestSender;
use uuid::Uuid;

#[test]
#[timeout(300)]
fn broadcast_sends_to_every_process() {
    // given
    let (tx, rx) = crossbeam_channel::unbounded();

    let processes = HashSet::from_iter(vec![Uuid::new_v4(), Uuid::new_v4()]);
    let mut stubborn_broadcast =
        build_stubborn_broadcast(Box::new(TestSender { tx }), processes.clone());

    // when
    stubborn_broadcast.broadcast(SystemBroadcastMessage {
        forwarder_id: Default::default(),
        message: SystemMessage {
            header: SystemMessageHeader {
                message_source_id: Default::default(),
                message_id: Default::default(),
            },
            data: SystemMessageContent { msg: vec![] },
        },
    });
    let target_processes = HashSet::from_iter(vec![rx.recv().unwrap(), rx.recv().unwrap()]);

    std::mem::drop(stubborn_broadcast);
    rx.recv().unwrap_err();

    // then
    assert_eq!(processes, target_processes);
}

#[test]
#[timeout(300)]
fn broadcast_tick() {
    // given
    let (tx, rx) = crossbeam_channel::unbounded();

    let processes = HashSet::from_iter(vec![Uuid::new_v4(), Uuid::new_v4()]);
    let mut stubborn_broadcast =
        build_stubborn_broadcast(Box::new(TestSender { tx }), processes.clone());

    // when
    stubborn_broadcast.broadcast(SystemBroadcastMessage {
        forwarder_id: Default::default(),
        message: SystemMessage {
            header: SystemMessageHeader {
                message_source_id: Default::default(),
                message_id: Default::default(),
            },
            data: SystemMessageContent { msg: vec![] },
        },
    });
    let target_processes1 = HashSet::from_iter(vec![rx.recv().unwrap(), rx.recv().unwrap()]);
    stubborn_broadcast.tick();
    let target_processes2 = HashSet::from_iter(vec![rx.recv().unwrap(), rx.recv().unwrap()]);

    std::mem::drop(stubborn_broadcast);
    rx.recv().unwrap_err();

    // then
    assert_eq!(processes, target_processes1);
    assert_eq!(processes, target_processes2);
}

#[test]
#[timeout(300)]
fn broadcast_receive_acknowledgment() {
    // given
    let (tx, rx) = crossbeam_channel::unbounded();

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let processes = HashSet::from_iter(vec![id1, id2]);
    let mut stubborn_broadcast =
        build_stubborn_broadcast(Box::new(TestSender { tx }), processes.clone());

    let msg = SystemMessage {
        header: SystemMessageHeader {
            message_source_id: Default::default(),
            message_id: Default::default(),
        },
        data: SystemMessageContent { msg: vec![] },
    };

    // when
    stubborn_broadcast.broadcast(SystemBroadcastMessage {
        forwarder_id: Default::default(),
        message: msg.clone(),
    });
    let target_processes1 = HashSet::from_iter(vec![rx.recv().unwrap(), rx.recv().unwrap()]);
    stubborn_broadcast.receive_acknowledgment(id1, msg.header);
    stubborn_broadcast.tick();
    let target_processes2 = HashSet::<Uuid>::from_iter(vec![rx.recv().unwrap()]);

    std::mem::drop(stubborn_broadcast);
    rx.recv().unwrap_err();

    // then
    assert_eq!(processes, target_processes1);
    assert_eq!(HashSet::from_iter(vec![id2].into_iter()), target_processes2);
}


#[test]
#[timeout(300)]
fn broadcast_send_acknowledgment() {
    // given
    let (tx, rx) = crossbeam_channel::unbounded();

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let processes = HashSet::from_iter(vec![id1, id2]);
    let mut stubborn_broadcast =
        build_stubborn_broadcast(Box::new(TestSender { tx }), processes.clone());

    let msg = SystemMessage {
        header: SystemMessageHeader {
            message_source_id: Default::default(),
            message_id: Default::default(),
        },
        data: SystemMessageContent { msg: vec![] },
    };

    // when
    stubborn_broadcast.broadcast(SystemBroadcastMessage {
        forwarder_id: Default::default(),
        message: msg.clone(),
    });
    let target_processes1 = HashSet::from_iter(vec![rx.recv().unwrap(), rx.recv().unwrap()]);
    stubborn_broadcast.send_acknowledgment(id2, SystemAcknowledgmentMessage{
        proc: Default::default(),
        hdr: msg.header,
    });
    let target_processes2 = HashSet::<Uuid>::from_iter(vec![rx.recv().unwrap()]);

    std::mem::drop(stubborn_broadcast);
    rx.recv().unwrap_err();

    // then
    assert_eq!(processes, target_processes1);
    assert_eq!(HashSet::from_iter(vec![id2].into_iter()), target_processes2);
}
