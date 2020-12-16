use actix::{Actor, Addr, Context, Handler, Message, Recipient};
use std::collections::HashSet;

/// Argument is the total number of processes, this means that size of the vector of readers
/// must be `processes - 1`. The leftover writer process is returned as the first element
/// of the tuple, so in total number of processes is `processes`.
pub async fn build_system(processes: usize) -> (Addr<WriterProcess>, Vec<Addr<ReaderProcess>>) {
    assert!(processes >= 1);
    let writer = build_writer().start();
    let mut pure_readers = Vec::new();

    for _ in 0..(processes - 1) {
        pure_readers.push(build_reader().start());
    }

    let mut all_processes: HashSet<Recipient<MessageBroadcast>> = HashSet::new();
    all_processes.insert(writer.clone().recipient());
    for reader in pure_readers.iter().cloned().map(|v| v.recipient()) {
        all_processes.insert(reader);
    }

    for reader in &pure_readers {
        reader.send(Init(all_processes.clone())).await.unwrap();
    }

    writer.send(Init(all_processes)).await.unwrap();

    (writer, pure_readers)
}

fn build_writer() -> WriterProcess {
    unimplemented!()
}

fn build_reader() -> ReaderProcess {
    unimplemented!()
}

/// Type stored in register.
#[derive(Default, Clone, Copy, Debug, Eq, PartialEq)]
pub struct Circle {
    pub x: i64,
    pub y: i64,
    pub r: u64,
}

type RegisterValue = Circle;

/// Add any fields you wish.
pub struct WriterProcess;
/// Add any fields you wish. `Default` implementation is necessary for tests.
#[derive(Default)]
pub struct ReaderProcess;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Read {
    pub read_return_callback: Box<dyn Fn(Circle) + Send>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Write {
    pub value: Circle,
    pub write_return_callback: Box<dyn Fn() + Send>,
}

#[derive(Message)]
#[rtype(result = "()")]
/// INIT - in our case, it contains references to all other processes in the system.
/// Size of this set is the number of processes in the system.
pub struct Init(pub HashSet<Recipient<MessageBroadcast>>);

/// You can add any handlers you need, the following ones will be used by tests.

impl Actor for WriterProcess {
    type Context = Context<Self>;
}

impl Actor for ReaderProcess {
    type Context = Context<Self>;
}

impl Handler<Write> for WriterProcess {
    type Result = ();

    fn handle(&mut self, _: Write, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<Read> for WriterProcess {
    type Result = ();

    fn handle(&mut self, _: Read, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<Init> for WriterProcess {
    type Result = ();

    fn handle(&mut self, _: Init, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<Read> for ReaderProcess {
    type Result = ();

    fn handle(&mut self, _: Read, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<Init> for ReaderProcess {
    type Result = ();

    fn handle(&mut self, _: Init, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<MessageBroadcast> for WriterProcess {
    type Result = ();

    fn handle(&mut self, _: MessageBroadcast, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

impl Handler<MessageBroadcast> for ReaderProcess {
    type Result = ();

    fn handle(&mut self, _: MessageBroadcast, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}

/// The following types and definitions are useful, but they are not part of public interface.
/// If you need some type defined here with private visibility to be public, just go ahead
/// and make it more visible. As always, do not export less public types than in template.

#[derive(Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq, Hash)]
/// Timestamp type.
struct Timestamp {
    value: u128,
}

/// Structures below are internal messages of the system - fill out the fields where necessary.
/// Their names and meaning comes from description of the algorithm.

#[derive(Message)]
#[rtype(result = "()")]
/// Reply to WRITE message - ACK.
struct WriteAck {}

#[derive(Message)]
#[rtype(result = "()")]
/// Broadcast message - either write or read. WARNING. This is public, although you
/// can define any fields you want.
pub enum MessageBroadcast {
    WriteBroadcast,
    ReadBroadcast,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Reply to READ message.
struct ReadValue {}
