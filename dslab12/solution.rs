use std::collections::HashSet;

use actix::{Actor, Addr, Context, Handler, Message, MessageResult};
use serde::{Deserialize, Serialize};

use crate::raft::{LogEntry, Raft, ClientRequest};
use bincode;

pub(crate) struct DistributedSet {
    /// Leader of Raft distributed log.
    raft: Addr<Raft>,
    /// Channel (from Raft's leader) to listen for committed entries.
    committed_entries: async_channel::Receiver<LogEntry>,
    /// The current (committed) state of the set.
    integers: HashSet<i64>,
}

impl DistributedSet {
    pub(crate) fn new(
        raft: Addr<Raft>,
        committed_entries: async_channel::Receiver<LogEntry>,
    ) -> Self {

        Self {
            raft,
            committed_entries,
            integers: HashSet::new(),
        }
    }

    /// Updates the current state of the set by applying the operation.
    fn apply(&mut self, op: SetOperation) {
        match op {
            SetOperation::Add(num) => {
                self.integers.insert(num);
            },
            SetOperation::Remove(num) => {
                self.integers.remove(&num);
            }
        }
    }
}

impl Actor for DistributedSet {
    type Context = Context<Self>;
}

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub(crate) enum SetOperation {
    Add(i64),
    Remove(i64),
}

#[derive(Message, Clone)]
#[rtype(result = "bool")]
pub(crate) struct IsPresent(pub(crate) i64);

impl Handler<SetOperation> for DistributedSet {
    type Result = ();

    fn handle(&mut self, msg: SetOperation, _: &mut Self::Context) -> Self::Result {
        self.raft.do_send(ClientRequest(bincode::serialize(&msg).unwrap()));
    }
}

impl Handler<IsPresent> for DistributedSet {
    type Result = MessageResult<IsPresent>;

    fn handle(&mut self, msg: IsPresent, _: &mut Self::Context) -> Self::Result {
        while let Ok(LogEntry{data, ..}) = self.committed_entries.try_recv() {
            let cmd : SetOperation = bincode::deserialize(&data).unwrap();
            self.apply(cmd.clone());
        }
        match self.integers.get(&msg.0) {
            None => MessageResult(false),
            Some(_) => MessageResult(true),
        }
    }
}
