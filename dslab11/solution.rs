use std::time::Duration;

use actix::{Actor, ActorContext, AsyncContext, Context, Handler, Message, SpawnHandle};
use std::collections::HashSet;
use std::iter::FromIterator;
use uuid::Uuid;

/// This state must be kept in stable storage, and updated before replying to messages.
#[derive(Default, Clone, Copy)]
pub(crate) struct ProcessState {
    /// 0 at boot.
    pub(crate) current_term: u64,
    /// Identifier of process which has received vote, or None if process has not voted
    /// in this term.
    voted_for: Option<Uuid>,
    /// Identifier of process which is thought to be the leader.
    leader_id: Option<Uuid>,
}

#[derive(Copy, Clone)]
pub(crate) struct ProcessConfig {
    pub(crate) self_id: Uuid,
    pub(crate) election_timeout: Duration,
    pub(crate) processes_count: usize,
}

#[derive(Message, Copy, Clone)]
#[rtype(result = "()")]
pub(crate) struct RaftMessage {
    pub(crate) header: RaftMessageHeader,
    pub(crate) content: RaftMessageContent,
}

#[derive(Message, Copy, Clone)]
#[rtype(result = "()")]
pub(crate) struct Shutdown {}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub(crate) struct RaftMessageHeader {
    /// Term of the process which issues message.
    pub(crate) term: u64,
}

#[derive(Copy, Clone)]
pub(crate) enum RaftMessageContent {
    Heartbeat {
        /// Id of the process issuing the message, which thinks of itself as leader.
        leader_id: Uuid,
    },
    HeartbeatResponse,
    RequestVote {
        candidate_id: Uuid,
    },
    RequestVoteResponse {
        /// Whether the vote was granted or not.
        granted: bool,
        /// Message source identifier.
        source: Uuid,
    },
}

pub(crate) trait StableStorage {
    fn put(&mut self, state: &ProcessState);

    fn get(&self) -> Option<ProcessState>;
}

pub(crate) trait Sender {
    fn send(&self, target: &Uuid, msg: RaftMessage);

    fn broadcast(&self, msg: RaftMessage);
}

pub struct Raft {
    state: ProcessState,
    config: ProcessConfig,
    storage: Box<dyn StableStorage>,
    sender: Box<dyn Sender>,
    /// Type of process.
    process_type: ProcessType,
    timer_handle: Option<SpawnHandle>,
}

// We give you some methods implementing larger chunks of logic.
impl Raft {
    pub(crate) fn new(
        config: ProcessConfig,
        storage: Box<dyn StableStorage>,
        sender: Box<dyn Sender>,
    ) -> Self {
        let state = storage.get().unwrap_or_default();
        Self {
            state,
            config,
            storage,
            sender,
            process_type: Default::default(),
            timer_handle: None,
        }
    }

    /// Sets term to higher number.
    fn update_term(&mut self, new_term: u64) {
        assert!(self.state.current_term < new_term);
        self.state.current_term = new_term;
        self.state.voted_for = None;
        self.state.leader_id = None;
        // No durable state update called here, must be called separately.
    }

    /// Durably saves state.
    fn update_state(&mut self) {
        self.storage.put(&self.state);
    }

    /// Handles received heartbeat.
    fn heartbeat(&mut self, leader_id: Uuid, leader_term: u64) {
        if leader_term >= self.state.current_term {
            self.state.leader_id = Some(leader_id);
            self.update_state();

            // Volatile state update
            match &mut self.process_type {
                ProcessType::Follower {
                    msgs_during_timeout,
                } => {
                    *msgs_during_timeout += 1;
                }
                ProcessType::Leader => {
                    log::debug!("Ignore")
                }
                _ => panic!("Invalid state"),
            };
        }

        // Response is sent always, so term information is disseminated.
        self.sender.send(
            &leader_id,
            RaftMessage {
                header: RaftMessageHeader {
                    term: self.state.current_term,
                },
                content: RaftMessageContent::HeartbeatResponse,
            },
        );
    }

    /// Starts election, run by follower or candidate.
    fn start_election(&mut self, ctx: &mut <Raft as Actor>::Context) {
        self.process_type = ProcessType::Candidate {
            votes_received: HashSet::from_iter(vec![self.config.self_id]),
        };
        self.update_term(self.state.current_term + 1);
        self.state.voted_for = Some(self.config.self_id);
        self.update_state();

        if let Some(timer_handle) = self.timer_handle.take() {
            ctx.cancel_future(timer_handle);
        }
        self.timer_handle = Some(ctx.run_interval(self.config.election_timeout, |raft, ctx| {
            raft.timeout(ctx);
        }));

        self.sender.broadcast(RaftMessage {
            header: RaftMessageHeader {
                term: self.state.current_term,
            },
            content: RaftMessageContent::RequestVote {
                candidate_id: self.config.self_id,
            },
        });
    }

    /// Common message processing.
    fn msg_received(&mut self, msg: &RaftMessage) {
        if msg.header.term > self.state.current_term {
            self.update_term(msg.header.term);
            self.process_type = ProcessType::Follower {
                msgs_during_timeout: 0,
            };
        }
    }

    /// Broadcasts heartbeat.
    fn broadcast_heartbeat(&mut self) {
        self.sender.broadcast(RaftMessage {
            header: RaftMessageHeader {
                term: self.state.current_term,
            },
            content: RaftMessageContent::Heartbeat {
                leader_id: self.config.self_id,
            },
        });
    }

    /// Handles timer timeout.
    fn timeout(&mut self, ctx: &mut <Raft as Actor>::Context) {
        match &mut self.process_type {
            ProcessType::Follower { .. } => {
                unimplemented!();
            }
            ProcessType::Candidate { .. } => {
                unimplemented!();
            }
            ProcessType::Leader => {
                unimplemented!();
            }
        }
    }
}

impl Actor for Raft {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.timer_handle = Some(ctx.run_interval(self.config.election_timeout, |raft, ctx| {
            raft.timeout(ctx);
        }));
    }
}

impl Handler<RaftMessage> for Raft {
    type Result = ();

    fn handle(&mut self, msg: RaftMessage, _ctx: &mut Self::Context) -> Self::Result {
        // Common state update.
        self.msg_received(&msg);

        // TODO message specific processing. Heartbeat is given as an example.
        match (&mut self.process_type, msg.content) {
            (_, RaftMessageContent::Heartbeat { leader_id }) => {
                self.heartbeat(leader_id, msg.header.term);
            }
            _ => {
                unimplemented!();
            }
        }
    }
}

impl Handler<Shutdown> for Raft {
    type Result = ();

    fn handle(&mut self, _: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

enum ProcessType {
    Follower { msgs_during_timeout: usize },
    Candidate { votes_received: HashSet<Uuid> },
    Leader,
}

impl Default for ProcessType {
    fn default() -> Self {
        ProcessType::Follower {
            msgs_during_timeout: 0,
        }
    }
}
