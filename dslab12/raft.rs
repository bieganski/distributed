use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::ops::Div;
use std::time::Duration;

use actix::{Actor, ActorContext, AsyncContext, Context, Handler, Message, SpawnHandle};
use uuid::Uuid;

/// This state must be kept in stable storage, and updated before replying to messages.
#[derive(Clone)]
pub(crate) struct ProcessDurableState {
    /// 0 at boot
    pub(crate) current_term: u64,
    /// Identifier of process which has received vote, or None if process has not voted
    /// in this term.
    pub(crate) voted_for: Option<Uuid>,
    /// Ordered log entries. First entry is an empty one.
    pub(crate) log: Vec<LogEntry>,
}

impl Default for ProcessDurableState {
    fn default() -> Self {
        ProcessDurableState {
            current_term: 0,
            voted_for: None,
            log: vec![LogEntry::default()],
        }
    }
}

#[derive(Message, Copy, Clone)]
#[rtype(result = "()")]
/// This one is for tests.
pub(crate) struct Shutdown {}

impl Handler<Shutdown> for Raft {
    type Result = ();

    fn handle(&mut self, _: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

#[derive(Default, Copy, Clone)]
struct ProcessVolatileState {
    /// Index of highest log entry know to be committed. Initialized to 0,
    /// increased monotonically.
    commit_index: u64,
}

#[derive(Clone)]
/// Such configuration in principle can also managed by Raft itself (changing process identifiers
/// at runtime could be tricky though). Our simplified Raft takes those for granted.
pub(crate) struct ProcessConfig {
    pub(crate) self_id: Uuid,
    pub(crate) election_timeout: Duration,
    pub(crate) processes: Vec<Uuid>,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub(crate) struct RaftMessage {
    pub(crate) header: RaftMessageHeader,
    pub(crate) content: RaftMessageContent,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub(crate) struct RaftMessageHeader {
    /// Term of the process which issues message.
    pub(crate) term: u64,
    /// Identifier of message source.
    pub(crate) source: Uuid,
}

#[derive(Default, Clone)]
pub(crate) struct LogEntry {
    /// Term when it was received by leader.
    pub(crate) term: u64,
    /// It is up to application to decide what meaning will stored data have. Raft doesn't care.
    /// Raft does not care about state machine, too. It comes in handy to snapshot state
    /// of the log.
    pub(crate) data: Vec<u8>,
}

#[derive(Clone)]
pub(crate) struct RequestVoteArgs {
    pub(crate) last_log_index: u64,
    pub(crate) last_log_term: u64,
}

#[derive(Clone)]
pub(crate) struct AppendEntriesArgs {
    pub(crate) prev_log_index: u64,
    pub(crate) prev_log_term: u64,
    pub(crate) entries: Vec<LogEntry>,
    pub(crate) leader_commit: u64,
}

#[derive(Clone)]
pub(crate) enum RaftMessageContent {
    AppendEntries(AppendEntriesArgs),
    AppendEntriesResponse {
        success: bool,
        /// Log index of the process (follower) after operation.
        log_index: u64,
    },
    RequestVote(RequestVoteArgs),
    RequestVoteResponse {
        /// Whether the vote was granted or not.
        granted: bool,
    },
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub(crate) struct ClientRequest(pub(crate) Vec<u8>);

pub(crate) trait StableStorage {
    fn put(&mut self, state: &ProcessDurableState);

    fn get(&self) -> Option<ProcessDurableState>;
}

pub(crate) trait Sender {
    fn send(&self, target: &Uuid, msg: RaftMessage);

    fn broadcast(&self, msg: RaftMessage);
}

pub struct Raft {
    committed_entries: async_channel::Sender<LogEntry>,
    state: ProcessDurableState,
    volatile_state: ProcessVolatileState,
    config: ProcessConfig,
    storage: Box<dyn StableStorage>,
    sender: Box<dyn Sender>,
    /// type of process
    process_type: ProcessType,
    timer_handle: Option<SpawnHandle>,
}

impl Actor for Raft {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.timer_handle = Some(ctx.run_interval(self.config.election_timeout, |raft, _| {
            raft.timeout();
        }));
    }
}

impl Handler<RaftMessage> for Raft {
    type Result = ();

    fn handle(&mut self, msg: RaftMessage, ctx: &mut Self::Context) -> Self::Result {
        self.msg_received(&msg);
        self.process_msg(msg, ctx);
    }
}

impl Handler<ClientRequest> for Raft {
    type Result = ();

    fn handle(&mut self, msg: ClientRequest, _: &mut Self::Context) -> Self::Result {
        if let ProcessType::Leader { match_index, .. } = &mut self.process_type {
            *match_index.get_mut(&self.config.self_id).unwrap() += 1;
            self.state.log.push(LogEntry {
                term: self.state.current_term,
                data: msg.0,
            });
            self.update_state();
            if self.config.processes.len() > 1 {
                self.broadcast_append_entries();
            } else {
                self.inc_committed_idx();
            }
        } else {
            log::warn!("Only leader can handle client requests!");
        }
    }
}

impl Raft {
    pub(crate) fn new(
        // This is output sink for committed entries by your Raft process.
        committed_entries: async_channel::Sender<LogEntry>,
        // Immutable configuration. Note that in a full version of Raft, this in principle could
        // be mutated during runtime.
        config: ProcessConfig,
        // Use this object as stable storage. Save state if it changed before replying
        // to messages.
        storage: Box<dyn StableStorage>,
        // This object can be used to send messages to other Raft processes in the system.
        sender: Box<dyn Sender>,
    ) -> Self {
        let state = storage.get().unwrap_or_default();

        Self {
            committed_entries,
            state,
            volatile_state: Default::default(),
            config,
            storage,
            sender,
            process_type: Default::default(),
            timer_handle: None,
        }
    }

    fn msg_header(&self) -> RaftMessageHeader {
        RaftMessageHeader {
            term: self.state.current_term,
            source: self.config.self_id,
        }
    }

    fn process_msg(&mut self, msg: RaftMessage, ctx: &mut <Raft as Actor>::Context) {
        match (&mut self.process_type, msg.content) {
            (_, RaftMessageContent::AppendEntries(args)) => {
                self.append_entries(msg.header, args);
            }
            (_, RaftMessageContent::RequestVote(args)) => {
                self.vote_requested(msg.header, args);
            }
            (
                ProcessType::Candidate { votes_received },
                RaftMessageContent::RequestVoteResponse { granted },
            ) => {
                if granted {
                    votes_received.insert(msg.header.source);
                }
                if votes_received.len() > self.config.processes.len() / 2 {
                    log::info!(
                        "Process {} is transitioning to leader for term {}",
                        self.config.self_id,
                        self.state.current_term
                    );
                    self.process_type = ProcessType::Leader {
                        next_index: self
                            .config
                            .processes
                            .iter()
                            .map(|ident| (*ident, self.log_idx() + 1))
                            .collect(),
                        match_index: self
                            .config
                            .processes
                            .iter()
                            .map(|ident| {
                                if ident == &self.config.self_id {
                                    (*ident, self.log_idx())
                                } else {
                                    (*ident, 0)
                                }
                            })
                            .collect(),
                    };
                    self.update_state();
                    self.broadcast_append_entries();
                    if let Some(timer_handle) = self.timer_handle.take() {
                        ctx.cancel_future(timer_handle);
                    }
                    self.timer_handle = Some(ctx.run_interval(
                        self.config.election_timeout.div(5),
                        |raft, _| {
                            raft.timeout();
                        },
                    ));
                    self.timeout();
                }
            }
            (ProcessType::Leader { .. }, RaftMessageContent::RequestVoteResponse { .. }) => {}
            (
                ProcessType::Leader {
                    next_index,
                    match_index,
                },
                RaftMessageContent::AppendEntriesResponse { success, log_index },
            ) => {
                if success {
                    *next_index.get_mut(&msg.header.source).unwrap() = log_index + 1;
                    *match_index.get_mut(&msg.header.source).unwrap() = log_index;
                    let next_commit_idx = Raft::progress_commit_index(
                        self.config.processes.len(),
                        &match_index,
                        &self.state,
                        self.volatile_state.commit_index,
                    );

                    let entries = self.state.log[(self.volatile_state.commit_index + 1) as usize
                        ..=next_commit_idx as usize]
                        .to_owned();

                    let committed_entries_ch = self.committed_entries.clone();
                    actix::spawn(async move {
                        for e in entries {
                            let _ = committed_entries_ch.send(e).await;
                        }
                    });
                    self.volatile_state.commit_index = next_commit_idx;
                } else {
                    *next_index.get_mut(&msg.header.source).unwrap() -= 1;
                    self.send_append_entries(&msg.header.source);
                }
            }
            _ => {
                log::trace!("Skipping message irrelevant for protocol");
            }
        }
    }

    fn update_term(&mut self, new_term: u64) {
        self.state.current_term = new_term;
        self.state.voted_for = None;
    }

    fn update_state(&mut self) {
        self.storage.put(&self.state);
    }

    fn append_entries(&mut self, hdr: RaftMessageHeader, args: AppendEntriesArgs) {
        let RaftMessageHeader {
            source: leader_id,
            term: leader_term,
        } = hdr;
        if leader_term < self.state.current_term
            || Some(args.prev_log_term)
                != self
                    .state
                    .log
                    .get(args.prev_log_index as usize)
                    .map(|e| e.term)
        {
            self.sender.send(
                &leader_id,
                RaftMessage {
                    header: self.msg_header(),
                    content: RaftMessageContent::AppendEntriesResponse {
                        success: false,
                        log_index: self.log_idx(),
                    },
                },
            );
            return;
        }

        /* Check for conflicts with local entries */
        for (pos, entry) in args.entries.iter().enumerate() {
            let local_log_idx = args.prev_log_index as usize + pos + 1;
            let local_entry = self.state.log.get(local_log_idx);
            match local_entry {
                Some(local_entry) => {
                    if local_entry.term != entry.term {
                        self.state.log.resize_with(local_log_idx, Default::default);
                        break;
                    }
                }
                None => break,
            }
        }

        /* Append entries not already in the log */
        let leader_log_idx = (args.prev_log_index as usize + args.entries.len()) as u64;
        if leader_log_idx > self.log_idx() {
            self.state.log.extend(
                args.entries
                    .into_iter()
                    .skip((self.log_idx() - args.prev_log_index) as usize),
            );
            self.update_state();
        }

        /* Update commit index */
        self.progress_committed_idx(std::cmp::min(
            args.leader_commit,
            self.state.log.len() as u64,
        ));

        match &mut self.process_type {
            ProcessType::Follower {
                msgs_during_timeout,
            } => {
                *msgs_during_timeout += 1;
            }
            ProcessType::Candidate { .. } => {
                self.process_type = ProcessType::Follower {
                    msgs_during_timeout: 1,
                };
            }
            _ => {
                panic!("Leader for current term received append entries!")
            }
        };
        self.sender.send(
            &leader_id,
            RaftMessage {
                header: self.msg_header(),
                content: RaftMessageContent::AppendEntriesResponse {
                    success: true,
                    log_index: self.log_idx(),
                },
            },
        );
    }

    fn inc_committed_idx(&mut self) {
        self.volatile_state.commit_index += 1;
        let entry = self
            .state
            .log
            .get(self.volatile_state.commit_index as usize)
            .unwrap()
            .clone();
        let committed_entries = self.committed_entries.clone();
        actix::spawn(async move {
            let _ = committed_entries.send(entry).await;
        });
    }

    fn progress_committed_idx(&mut self, target: u64) {
        while self.volatile_state.commit_index < target {
            self.inc_committed_idx();
        }
    }

    fn log_idx(&self) -> u64 {
        self.state.log.len() as u64 - 1
    }

    fn candidate_has_fresh_log(&self, candidate_last_log_entry: (u64, u64)) -> bool {
        candidate_last_log_entry >= (self.state.log.last().unwrap().term, self.log_idx())
    }

    fn vote_requested(&mut self, hdr: RaftMessageHeader, args: RequestVoteArgs) {
        let RaftMessageHeader {
            term: request_term,
            source: candidate_id,
        } = hdr;
        let mut granted = false;

        if (self.state.voted_for == None || self.state.voted_for == Some(candidate_id))
            && request_term >= self.state.current_term
            && self.candidate_has_fresh_log((args.last_log_term, args.last_log_index))
        {
            granted = true;
            self.state.voted_for = Some(candidate_id);
            self.update_state();
        }

        self.sender.send(
            &candidate_id,
            RaftMessage {
                header: self.msg_header(),
                content: RaftMessageContent::RequestVoteResponse { granted },
            },
        );
    }

    fn start_election(&mut self) {
        self.process_type = ProcessType::Candidate {
            votes_received: HashSet::from_iter(vec![self.config.self_id]),
        };
        self.update_term(self.state.current_term + 1);
        self.state.voted_for = Some(self.config.self_id);
        self.update_state();

        self.sender.broadcast(RaftMessage {
            header: self.msg_header(),
            content: RaftMessageContent::RequestVote(RequestVoteArgs {
                last_log_index: self.state.log.len() as u64 - 1,
                last_log_term: self.state.log.last().unwrap().term,
            }),
        });
    }

    fn msg_received(&mut self, msg: &RaftMessage) {
        if msg.header.term > self.state.current_term {
            self.update_term(msg.header.term);
            self.process_type = ProcessType::Follower {
                msgs_during_timeout: 1,
            };
        }
    }

    fn broadcast_append_entries(&mut self) {
        for process in &self.config.processes {
            if process == &self.config.self_id {
                continue;
            }
            let msg = self.prepare_append_entries_message(process);
            self.sender.send(process, msg);
        }
    }

    fn send_append_entries(&mut self, target: &Uuid) {
        let msg = self.prepare_append_entries_message(target);
        self.sender.send(target, msg);
    }

    fn prepare_append_entries_message(&self, process: &Uuid) -> RaftMessage {
        if let ProcessType::Leader { next_index, .. } = &self.process_type {
            let next_index = *next_index.get(process).unwrap();
            let prev_log_index = next_index - 1;

            RaftMessage {
                header: self.msg_header(),
                content: RaftMessageContent::AppendEntries(AppendEntriesArgs {
                    prev_log_index,
                    prev_log_term: self.state.log.get(prev_log_index as usize).unwrap().term,
                    entries: self.state.log[next_index as usize..].to_vec(),
                    leader_commit: self.volatile_state.commit_index,
                }),
            }
        } else {
            panic!("Only leader can send append entries");
        }
    }

    fn progress_commit_index(
        processes_count: usize,
        match_index: &HashMap<Uuid, u64>,
        state: &ProcessDurableState,
        current_commit_idx: u64,
    ) -> u64 {
        /* Inefficient implementation */
        let mut idx = state.log.len() as u64;

        while idx > current_commit_idx {
            idx -= 1;
            if state.log.get(idx as usize).map(|e| e.term) == Some(state.current_term)
                && match_index
                    .values()
                    .filter(|process_idx| **process_idx >= idx)
                    .count()
                    > processes_count / 2
            {
                return idx;
            }
        }

        current_commit_idx
    }

    fn timeout(&mut self) {
        match &mut self.process_type {
            ProcessType::Follower {
                msgs_during_timeout,
            } => {
                if *msgs_during_timeout == 0 {
                    self.start_election();
                } else {
                    *msgs_during_timeout = 0;
                }
            }
            ProcessType::Candidate { .. } => {
                self.start_election();
            }
            ProcessType::Leader { .. } => {
                self.broadcast_append_entries();
            }
        }
    }
}

enum ProcessType {
    Follower {
        msgs_during_timeout: usize,
    },
    Candidate {
        votes_received: HashSet<Uuid>,
    },
    Leader {
        next_index: HashMap<Uuid, u64>,
        match_index: HashMap<Uuid, u64>,
    },
}

impl Default for ProcessType {
    fn default() -> Self {
        ProcessType::Follower {
            msgs_during_timeout: 0,
        }
    }
}

pub(crate) mod utils {
    use super::{ProcessConfig, ProcessDurableState, Raft, RaftMessage, StableStorage};
    use crate::raft::LogEntry;
    use actix::clock::Duration;
    use actix::{Actor, Addr, Recipient};
    use async_channel::{unbounded, Receiver};
    use std::collections::HashMap;
    use std::ops::DerefMut;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    pub(crate) fn build_process(
        election_timeout: Duration,
        processes: &[Uuid],
        self_id: Uuid,
        sender: Box<dyn super::Sender>,
        storage: Box<dyn StableStorage>,
    ) -> (Addr<Raft>, Receiver<LogEntry>) {
        let (tx, rx) = unbounded();
        let config = ProcessConfig {
            self_id,
            election_timeout,
            processes: processes.to_owned(),
        };

        (Raft::new(tx, config, storage, sender).start(), rx)
    }

    #[derive(Clone, Default)]
    pub(crate) struct ActixSender {
        processes: Arc<Mutex<HashMap<Uuid, Recipient<RaftMessage>>>>,
    }

    impl ActixSender {
        pub(crate) fn insert(&self, id: Uuid, addr: Recipient<RaftMessage>) {
            self.processes.lock().unwrap().insert(id, addr);
        }

        pub(crate) fn remove(&self, id: &Uuid) {
            self.processes.lock().unwrap().remove(id);
        }
    }

    impl super::Sender for ActixSender {
        fn send(&self, target: &Uuid, msg: RaftMessage) {
            if let Some(addr) = self.processes.lock().unwrap().get(target) {
                let addr = addr.clone();
                actix::spawn(async move { addr.send(msg).await.unwrap() });
            }
        }

        fn broadcast(&self, msg: RaftMessage) {
            let map = self.processes.lock().unwrap();
            for (_, addr) in map.iter() {
                let addr = addr.clone();
                let msg = msg.clone();
                actix::spawn(async move { addr.send(msg).await.unwrap() });
            }
        }
    }

    #[derive(Default, Clone)]
    pub(crate) struct RamStorage {
        state: Arc<Mutex<Option<ProcessDurableState>>>,
    }

    impl StableStorage for RamStorage {
        fn put(&mut self, state: &ProcessDurableState) {
            *self.state.lock().unwrap().deref_mut() = Some(state.clone());
        }

        fn get(&self) -> Option<ProcessDurableState> {
            self.state.lock().unwrap().clone()
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::utils::*;
    use super::*;
    use actix::clock::Duration;
    use actix::{Actor, Addr, Context, Handler};
    use async_channel::{unbounded, Sender};
    use ntest::timeout;
    use uuid::Uuid;

    #[actix_rt::test]
    #[timeout(700)]
    async fn leader_commits_under_ideal_circumstances() {
        // given
        let entry_data = vec![1, 2, 3, 4, 5];
        let ident_leader = Uuid::new_v4();
        let ident_follower = Uuid::new_v4();
        let processes = vec![ident_leader, ident_follower];
        let sender = ActixSender::default();

        let (leader_tx, _) = unbounded();
        let raft_leader = Raft::new(
            leader_tx,
            ProcessConfig {
                self_id: ident_leader,
                election_timeout: Duration::from_millis(100),
                processes: processes.clone(),
            },
            Box::new(RamStorage::default()),
            Box::new(sender.clone()),
        )
        .start();
        let (follower_tx, follower_rx) = unbounded();
        let raft_follower = Raft::new(
            follower_tx,
            ProcessConfig {
                self_id: ident_follower,
                election_timeout: Duration::from_millis(300),
                processes,
            },
            Box::new(RamStorage::default()),
            Box::new(sender.clone()),
        )
        .start();
        sender.insert(ident_leader, raft_leader.clone().recipient());
        sender.insert(ident_follower, raft_follower.recipient());
        actix::clock::delay_for(Duration::from_millis(200)).await;

        // when
        raft_leader
            .send(ClientRequest(entry_data.clone()))
            .await
            .unwrap();

        // then
        assert_eq!(entry_data, follower_rx.recv().await.unwrap().data);
    }

    #[actix_rt::test]
    #[timeout(1000)]
    async fn leader_commits_after_distributing_entries() {
        // given
        let (tx, rx) = unbounded();
        let entry_data = vec![1, 2, 3, 4, 5];
        let ident_leader = Uuid::new_v4();
        let ident_follower = Uuid::new_v4();
        let processes = vec![ident_leader, ident_follower];
        let sender = ActixSender::default();

        let (leader_tx, _) = unbounded();
        let raft_leader = Raft::new(
            leader_tx,
            ProcessConfig {
                self_id: ident_leader,
                election_timeout: Duration::from_millis(100),
                processes: processes.clone(),
            },
            Box::new(RamStorage::default()),
            Box::new(sender.clone()),
        )
        .start();
        let (follower_tx, _) = unbounded();
        let raft_follower = RaftSpy::new(
            Raft::new(
                follower_tx,
                ProcessConfig {
                    self_id: ident_follower,
                    election_timeout: Duration::from_millis(300),
                    processes,
                },
                Box::new(RamStorage::default()),
                Box::new(sender.clone()),
            ),
            tx,
        )
        .start();
        sender.insert(ident_leader, raft_leader.clone().recipient());
        sender.insert(ident_follower, raft_follower.recipient());
        actix::clock::delay_for(Duration::from_millis(200)).await;

        // when
        raft_leader
            .send(ClientRequest(entry_data.clone()))
            .await
            .unwrap();
        actix::clock::delay_for(Duration::from_millis(100)).await;

        // then
        let mut entries_message_detected = false;
        let follower_msgs = append_entries_from_leader(&extract_messages(&rx));
        for (_, args) in follower_msgs {
            if !entries_message_detected && args.entries.len() != 0 {
                entries_message_detected = true;
            }
            if entries_message_detected && args.leader_commit == 1 {
                // Exit test OK!
                return;
            }
        }
        panic!("Leader has not increased commit index");
    }

    #[actix_rt::test]
    #[timeout(1000)]
    async fn system_makes_progress_when_there_is_a_majority() {
        // given
        let entry_data = vec![1, 2, 3, 4, 5];
        let ident_leader = Uuid::new_v4();
        let ident_follower = Uuid::new_v4();
        let processes = vec![ident_leader, ident_follower, Uuid::new_v4()];
        let sender = ActixSender::default();

        let (leader_tx, _) = unbounded();
        let raft_leader = Raft::new(
            leader_tx,
            ProcessConfig {
                self_id: ident_leader,
                election_timeout: Duration::from_millis(100),
                processes: processes.clone(),
            },
            Box::new(RamStorage::default()),
            Box::new(sender.clone()),
        )
        .start();
        let (follower_tx, follower_rx) = unbounded();
        let raft_follower = Raft::new(
            follower_tx,
            ProcessConfig {
                self_id: ident_follower,
                election_timeout: Duration::from_millis(300),
                processes,
            },
            Box::new(RamStorage::default()),
            Box::new(sender.clone()),
        )
        .start();
        sender.insert(ident_leader, raft_leader.clone().recipient());
        sender.insert(ident_follower, raft_follower.recipient());
        actix::clock::delay_for(Duration::from_millis(200)).await;

        // when
        raft_leader
            .send(ClientRequest(entry_data.clone()))
            .await
            .unwrap();

        // then
        assert_eq!(entry_data, follower_rx.recv().await.unwrap().data);
    }

    #[actix_rt::test]
    #[timeout(2000)]
    async fn system_does_not_make_progress_without_majority() {
        // given
        let entry_data = vec![1, 2, 3, 4, 5];
        let ident_leader = Uuid::new_v4();
        let processes = vec![ident_leader, Uuid::new_v4(), Uuid::new_v4()];
        let sender = ActixSender::default();

        let (would_be_leader_tx, would_be_leader_rx) = unbounded();
        let raft_leader = Raft::new(
            would_be_leader_tx,
            ProcessConfig {
                self_id: ident_leader,
                election_timeout: Duration::from_millis(100),
                processes: processes.clone(),
            },
            Box::new(RamStorage::default()),
            Box::new(sender.clone()),
        )
        .start();
        sender.insert(ident_leader, raft_leader.clone().recipient());
        actix::clock::delay_for(Duration::from_millis(200)).await;

        // when
        raft_leader.send(ClientRequest(entry_data)).await.unwrap();

        // when
        actix::clock::delay_for(Duration::from_millis(1000)).await;

        // then
        assert!(would_be_leader_rx.is_empty());
    }

    #[actix_rt::test]
    #[timeout(500)]
    async fn follower_denies_vote_for_candidate_with_outdated_log() {
        // given
        let (tx, rx) = unbounded();
        let mut storage = RamStorage::default();
        storage.put(&ProcessDurableState {
            current_term: 3,
            voted_for: None,
            log: vec![
                LogEntry {
                    term: 0,
                    data: vec![],
                },
                LogEntry {
                    term: 2,
                    data: vec![],
                },
                LogEntry {
                    term: 2,
                    data: vec![],
                },
            ],
        });
        let other_ident = Uuid::new_v4();
        let ident_follower = Uuid::new_v4();
        let (follower_tx, _) = unbounded();
        let raft_follower = Raft::new(
            follower_tx,
            ProcessConfig {
                self_id: ident_follower,
                election_timeout: Duration::from_secs(10),
                processes: vec![other_ident, ident_follower],
            },
            Box::new(storage),
            Box::new(RamSender { tx }),
        )
        .start();

        // when

        // Older term of the last message.
        raft_follower
            .send(RaftMessage {
                header: RaftMessageHeader {
                    term: 1,
                    source: other_ident,
                },
                content: RaftMessageContent::RequestVote(RequestVoteArgs {
                    last_log_index: 2,
                    last_log_term: 1,
                }),
            })
            .await
            .unwrap();

        // Shorter log in candidate.
        raft_follower
            .send(RaftMessage {
                header: RaftMessageHeader {
                    term: 1,
                    source: other_ident,
                },
                content: RaftMessageContent::RequestVote(RequestVoteArgs {
                    last_log_index: 1,
                    last_log_term: 2,
                }),
            })
            .await
            .unwrap();

        // then
        assert!(matches!(rx.recv().await.unwrap(), RaftMessage {
            header: _,
            content: RaftMessageContent::RequestVoteResponse { granted: false }
        }));
        assert!(matches!(rx.recv().await.unwrap(), RaftMessage {
            header: _,
            content: RaftMessageContent::RequestVoteResponse { granted: false }
        }));
    }

    #[actix_rt::test]
    #[timeout(500)]
    async fn follower_rejects_inconsistent_append_entry() {
        // given
        let (tx, rx) = unbounded();
        let mut storage = RamStorage::default();
        storage.put(&ProcessDurableState {
            current_term: 2,
            voted_for: None,
            log: vec![
                LogEntry {
                    term: 0,
                    data: vec![],
                },
                LogEntry {
                    term: 1,
                    data: vec![],
                },
            ],
        });
        let other_ident = Uuid::new_v4();
        let ident_follower = Uuid::new_v4();
        let (follower_tx, _) = unbounded();
        let raft_follower = Raft::new(
            follower_tx,
            ProcessConfig {
                self_id: ident_follower,
                election_timeout: Duration::from_secs(10),
                processes: vec![other_ident, ident_follower],
            },
            Box::new(storage),
            Box::new(RamSender { tx }),
        )
        .start();

        // when
        raft_follower
            .send(RaftMessage {
                header: RaftMessageHeader {
                    term: 2,
                    source: other_ident,
                },
                content: RaftMessageContent::AppendEntries(AppendEntriesArgs {
                    prev_log_index: 1,
                    prev_log_term: 2,
                    entries: vec![LogEntry {
                        term: 2,
                        data: vec![],
                    }],
                    leader_commit: 1,
                }),
            })
            .await
            .unwrap();

        // then
        assert!(matches!(rx.recv().await.unwrap(), RaftMessage {
            header: _,
            content: RaftMessageContent::AppendEntriesResponse { success: false, log_index: 1 }
        }));
    }

    #[actix_rt::test]
    #[timeout(500)]
    async fn follower_overwrites_its_log_when_leader_issues_append_entries() {
        // given
        let (tx, rx) = unbounded();
        let mut storage = RamStorage::default();
        storage.put(&ProcessDurableState {
            current_term: 2,
            voted_for: None,
            log: vec![
                LogEntry {
                    term: 0,
                    data: vec![],
                },
                LogEntry {
                    term: 1,
                    data: vec![],
                },
            ],
        });
        let other_ident = Uuid::new_v4();
        let ident_follower = Uuid::new_v4();
        let (follower_tx, _) = unbounded();
        let raft_follower = Raft::new(
            follower_tx,
            ProcessConfig {
                self_id: ident_follower,
                election_timeout: Duration::from_secs(10),
                processes: vec![other_ident, ident_follower],
            },
            Box::new(storage.clone()),
            Box::new(RamSender { tx }),
        )
        .start();

        // when
        raft_follower
            .send(RaftMessage {
                header: RaftMessageHeader {
                    term: 2,
                    source: other_ident,
                },
                content: RaftMessageContent::AppendEntries(AppendEntriesArgs {
                    prev_log_index: 0,
                    prev_log_term: 0,
                    entries: vec![LogEntry {
                        term: 2,
                        data: vec![9, 4, 7],
                    }],
                    leader_commit: 1,
                }),
            })
            .await
            .unwrap();
        // Wait for response.
        rx.recv().await.unwrap();
        let effective_log = storage.get().unwrap().log;

        // then
        assert_eq!(effective_log.len(), 2);
        assert_eq!(effective_log.get(1).unwrap().term, 2);
        assert_eq!(effective_log.get(1).unwrap().data, vec![9, 4, 7]);
    }

    pub(crate) fn append_entries_from_leader(
        msgs: &Vec<RaftMessage>,
    ) -> Vec<(RaftMessageHeader, AppendEntriesArgs)> {
        let mut res = Vec::new();
        for msg in msgs {
            match &msg.content {
                RaftMessageContent::AppendEntries(args) => {
                    res.push((msg.header, args.clone()));
                }
                _ => {}
            }
        }
        res
    }

    pub(crate) fn extract_messages<T>(rx: &async_channel::Receiver<T>) -> Vec<T> {
        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        msgs
    }

    pub(crate) struct RaftSpy {
        pub(crate) raft: Addr<Raft>,
        pub(crate) tx: async_channel::Sender<RaftMessage>,
    }

    impl RaftSpy {
        pub(crate) fn new(raft: Raft, tx: async_channel::Sender<RaftMessage>) -> Self {
            let raft = raft.start();
            Self { raft, tx }
        }
    }

    impl Actor for RaftSpy {
        type Context = Context<Self>;

        fn stopped(&mut self, _: &mut Self::Context) {
            let raft = self.raft.clone();
            actix::spawn(async move { raft.send(Shutdown {}).await.unwrap() });
        }
    }

    impl Handler<RaftMessage> for RaftSpy {
        type Result = ();

        fn handle(&mut self, msg: RaftMessage, _ctx: &mut Self::Context) -> Self::Result {
            let raft = self.raft.clone();
            let tx = self.tx.clone();

            actix::spawn(async move {
                let _ = tx.send(msg.clone()).await;
                raft.send(msg).await.unwrap()
            });
        }
    }

    struct RamSender {
        tx: Sender<RaftMessage>,
    }

    impl super::Sender for RamSender {
        fn send(&self, _: &Uuid, msg: RaftMessage) {
            self.tx.try_send(msg).unwrap();
        }

        fn broadcast(&self, msg: RaftMessage) {
            self.tx.try_send(msg).unwrap();
        }
    }
}
