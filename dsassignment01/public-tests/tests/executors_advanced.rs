use assignment_1_solution::{Handler, System, Tick, ModuleRef};
use crossbeam_channel::{unbounded, Receiver, Sender};
use ntest::timeout;
use std::time::{Duration, SystemTime};

#[test]
#[timeout(300)]
fn counter_receives_messages() {
    let mut system = System::new();
    let (tx, rx) = unbounded();

    let ctr_ref = system.register_module(Counter::new(tx));

    for i in 1..3 {
        ctr_ref.send(Inc {});
        assert_eq!(i, rx.recv().unwrap());
    }
}

struct Counter {
    num: u64,
    tx: Sender<u64>,
}

impl Counter {
    fn new(tx: Sender<u64>) -> Self {
        Counter { num: 0, tx }
    }
}

#[derive(Debug, Clone)]
struct Inc {}

impl Handler<Inc> for Counter {
    fn handle(&mut self, _msg: Inc) {
        self.num += 1;
        self.tx.send(self.num).unwrap();
    }
}
fn are_close(mut tp1: SystemTime, mut tp2: SystemTime) -> bool {
    if tp1 > tp2 {
        std::mem::swap(&mut tp1, &mut tp2);
    }
    tp2.duration_since(tp1).unwrap() < Duration::from_millis(10)
}

fn next_msg_is_ok(rx: &Receiver<SystemTime>, start: SystemTime, milis_from_start: u64) -> bool {
    are_close(
        rx.recv().unwrap(),
        start
            .checked_add(Duration::from_millis(milis_from_start))
            .unwrap(),
    )
}

#[test]
#[timeout(1100)]
fn timer_test_one_timer() {
    let mut system = System::new();
    let (tx, rx) = unbounded();

    let ctr_ref = system.register_module(TimerTester { tx });
    let start = SystemTime::now();
    system.request_tick(&ctr_ref, Duration::from_millis(200));

    assert!(next_msg_is_ok(&rx, start, 200));
    assert!(next_msg_is_ok(&rx, start, 400));
    assert!(next_msg_is_ok(&rx, start, 600));
    assert!(next_msg_is_ok(&rx, start, 800));
    assert!(next_msg_is_ok(&rx, start, 1000));
}

#[test]
#[timeout(1100)]
fn timer_test_two_timers() {
    let mut system = System::new();
    let (tx, rx) = unbounded();

    let ctr_ref = system.register_module(TimerTester { tx });
    let start = SystemTime::now();
    system.request_tick(&ctr_ref, Duration::from_millis(300));
    system.request_tick(&ctr_ref, Duration::from_millis(200));

    assert!(next_msg_is_ok(&rx, start, 200));
    assert!(next_msg_is_ok(&rx, start, 300));
    assert!(next_msg_is_ok(&rx, start, 400));
    assert!(next_msg_is_ok(&rx, start, 600));
    assert!(next_msg_is_ok(&rx, start, 600));
    assert!(next_msg_is_ok(&rx, start, 800));
    assert!(next_msg_is_ok(&rx, start, 900));
    assert!(next_msg_is_ok(&rx, start, 1000));
}

struct TimerTester {
    tx: Sender<SystemTime>,
}

impl Handler<Tick> for TimerTester {
    fn handle(&mut self, _msg: Tick) {
        self.tx.send(SystemTime::now()).unwrap();
    }
}

#[test]
#[timeout(300)]
fn many_parallel_computations() {
    let mut system = System::new();
    let (tx, rx) = unbounded();

    let starter = system.register_module(Starter{});
    let a = system.register_module(Computator{
        next: None,
        tx: tx.clone(),
        name: "a",
    });
    let b = system.register_module(Computator{
        next: None,
        tx: tx.clone(),
        name: "b",
    });
    let c = system.register_module(Computator{
        next: None,
        tx: tx.clone(),
        name: "c",
    });
    let d = system.register_module(Computator{
        next: None,
        tx: tx.clone(),
        name: "d",
    });

    a.send(Init{next: b.clone()});
    b.send(Init{next: a.clone()});

    c.send(Init{next: d.clone()});
    d.send(Init{next: c.clone()});

    starter.send(Start {
        first: a,
        second: c,
    });

    for _ in 1..4 {
        assert_eq!(rx.recv().unwrap(), "a");
        assert_eq!(rx.recv().unwrap(), "c");
        assert_eq!(rx.recv().unwrap(), "b");
        assert_eq!(rx.recv().unwrap(), "d");
    }
}

struct Computator {
    next: Option<ModuleRef<Computator>>,
    tx: Sender<&'static str>,
    name: &'static str,
}

#[derive(Clone, Debug)]
struct Init {
    next: ModuleRef<Computator>
}

impl Handler<Init> for Computator {
    fn handle(&mut self, msg: Init) {
        self.next = Some(msg.next);
    }
}

#[derive(Clone, Debug)]
struct Turn {}

impl Handler<Turn> for Computator {
    fn handle(&mut self, msg: Turn) {
        let _ = self.tx.send(self.name);
        self.next.as_ref().unwrap().send(msg);
    }
}

struct Starter {}

#[derive(Clone, Debug)]
struct Start {
    first: ModuleRef<Computator>,
    second: ModuleRef<Computator>,
}

impl Handler<Start> for Starter {
    fn handle(&mut self, msg: Start) {
        msg.first.send(Turn{});
        msg.second.send(Turn{});
    }
}
