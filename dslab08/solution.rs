use uuid::Uuid;
use actix::{Actor, AsyncContext, Addr, Context, Handler, Message, Recipient};
use std::collections::{HashSet, HashMap};

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
    WriterProcess::default()
}

fn build_reader() -> ReaderProcess {
    ReaderProcess::default()
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
pub struct WriterProcess {
    id : Uuid,
    val : RegisterValue,
    wts : Timestamp,
    acks : usize,
    all_procs: HashSet<Recipient<MessageBroadcast>>,
    callback: Option<Box<dyn Fn() + Send>>,
}

type ReadListType = HashMap<Uuid, (Timestamp, RegisterValue)>;

/// Add any fields you wish. `Default` implementation is necessary for tests.
pub struct ReaderProcess {
    id : Uuid,
    ts : Timestamp,
    val : RegisterValue,
    rid : Timestamp,
    readlist : ReadListType,
    all_procs: HashSet<Recipient<MessageBroadcast>>,
    callback: Option<Box<dyn Fn(RegisterValue) + Send>>,
}

impl Default for ReaderProcess {
    fn default() -> Self {
        Self {
            id : Uuid::new_v4(),
            ts : Timestamp{value: 0},
            val : RegisterValue::default(),
            rid : Timestamp{value: 0},
            readlist : ReadListType::default(),
            all_procs: HashSet::default(),
            callback: None,
        }
    }
}

impl Default for WriterProcess {
    fn default() -> Self {
        Self {
            id : Uuid::new_v4(),
            val : RegisterValue::default(),
            wts : Timestamp{value: 0},
            all_procs: HashSet::default(),
            callback: None,
            acks: 0,
        }
    }
}

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

    fn handle(&mut self, msg: Write, cxt: &mut Self::Context) -> Self::Result {
        self.callback = Some(msg.write_return_callback);
        self.acks = 0;
        self.val = msg.value;
        self.wts.value += 1;
        for p in self.all_procs.iter() {
            p.do_send(MessageBroadcast::WriteBroadcast(msg.value, self.wts, cxt.address().recipient())).unwrap();
        }
    }
}

impl Handler<Read> for WriterProcess {
    type Result = ();

    fn handle(&mut self, msg: Read, _: &mut Self::Context) -> Self::Result {
        msg.read_return_callback.as_ref()(self.val);
    }
}

impl Handler<Init> for WriterProcess {
    type Result = ();

    fn handle(&mut self, init: Init, _: &mut Self::Context) -> Self::Result {
        self.all_procs = init.0;
    }
}

impl Handler<Read> for ReaderProcess {
    type Result = ();

    fn handle(&mut self, msg: Read, cxt: &mut Self::Context) -> Self::Result {
        self.callback = Some(msg.read_return_callback);
        self.rid.value += 1;
        for p in self.all_procs.iter() {
            p.do_send(MessageBroadcast::ReadBroadcast(self.rid, cxt.address().recipient())).unwrap();
        }
    }
}

impl Handler<Init> for ReaderProcess {
    type Result = ();

    fn handle(&mut self, msg: Init, _: &mut Self::Context) -> Self::Result {
        self.all_procs = msg.0;
    }
}


impl Handler<MessageBroadcast> for WriterProcess {
    type Result = ();

    fn handle(&mut self, msg: MessageBroadcast, _: &mut Self::Context) -> Self::Result {
        match msg {
            MessageBroadcast::ReadBroadcast(ts, recipient) => {
                let val = ReadValue{
                    from: self.id,
                    ts: self.wts,
                    value: self.val,
                    rid: ts,
                };
                recipient.do_send(val).unwrap();
            },
            MessageBroadcast::WriteBroadcast(_, _, me) => {
                me.do_send(WriteAck{ts: self.wts}).unwrap();
            },
        }
    }
}

impl Handler<MessageBroadcast> for ReaderProcess {
    type Result = ();

    fn handle(&mut self, msg: MessageBroadcast, _: &mut Self::Context) -> Self::Result {
        match msg {
            MessageBroadcast::ReadBroadcast(ts, recipient) => {
                let val = ReadValue{
                    from: self.id,
                    ts: self.ts,
                    value: self.val,
                    rid: ts,
                };
                recipient.do_send(val).unwrap();
            },
            
            MessageBroadcast::WriteBroadcast(val, ts, recipient) => {
                if ts > self.ts {
                    self.val = val;
                    self.ts = ts;
                }
                recipient.do_send(WriteAck{ts}).unwrap();
            },
        }
    }
}

fn highest_val(map : &ReadListType) -> (Timestamp, RegisterValue) {
    assert_ne!(map.len(), 0);
    *map
        .iter()
        .max_by(|a, b| ((a.1).0).cmp(&(b.1).0))
        .map(|(_k, v)| v).unwrap()
}

impl Handler<ReadValue> for ReaderProcess {
    type Result = ();
    fn handle(&mut self, msg: ReadValue, _: &mut Self::Context) -> Self::Result {
        if self.rid == msg.rid {
            self.readlist.insert(msg.from, (msg.ts, msg.value));
            if self.readlist.len() > (self.all_procs.len() / 2) {
                let (k, v) = highest_val(&self.readlist);
                self.readlist.drain();
                self.ts = k;
                self.val = v;
                self.callback.as_ref().unwrap()(v);
            }
        }
    }
}

impl Handler<WriteAck> for WriterProcess {
    type Result = ();

    fn handle(&mut self, msg: WriteAck, _: &mut Self::Context) -> Self::Result {
        if msg.ts != self.wts {
            return;
        }
        self.acks += 1;
        if self.acks > (self.all_procs.len() / 2) {
            self.acks = 0;
            self.callback.as_ref().unwrap()();
        }
    }
}

/// The following types and definitions are useful, but they are not part of public interface.
/// If you need some type defined here with private visibility to be public, just go ahead
/// and make it more visible. As always, do not export less public types than in template.

#[derive(Debug, Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq, Hash)]
/// Timestamp type.
pub struct Timestamp {
    value: u128,
}

/// Structures below are internal messages of the system - fill out the fields where necessary.
/// Their names and meaning comes from description of the algorithm.

#[derive(Message)]
#[rtype(result = "()")]
/// Reply to WRITE message - ACK.
pub struct WriteAck {
    ts: Timestamp,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Broadcast message - either write or read. WARNING. This is public, although you
/// can define any fields you want.
pub enum MessageBroadcast {
    WriteBroadcast(RegisterValue, Timestamp, Recipient<WriteAck>),
    ReadBroadcast(Timestamp, Recipient<ReadValue>),
}

#[derive(Message)]
#[rtype(result = "()")]
/// Reply to READ message.
pub struct ReadValue {
    from: Uuid,
    ts: Timestamp,
    value: RegisterValue,
    rid: Timestamp,
}
