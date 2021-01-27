use std::collections::HashSet;

use actix::{Actor, Addr, Context, Handler, Message, MessageResult};
use serde::{Deserialize, Serialize};

use crate::raft::{LogEntry, Raft, ClientRequest};

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
        unimplemented!()
        // Start with an empty set.
    }

    /// Updates the current state of the set by applying the operation.
    fn apply(&mut self, op: SetOperation) {
        unimplemented!()
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
        unimplemented!()
    }
}

impl Handler<IsPresent> for DistributedSet {
    type Result = MessageResult<IsPresent>;

    fn handle(&mut self, msg: IsPresent, _: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}
