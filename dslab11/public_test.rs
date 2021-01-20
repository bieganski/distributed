#[cfg(test)]
pub(crate) mod tests {
    use actix::clock::Duration;
    use actix::{Actor, Addr, Context, Handler};
    use async_channel::unbounded;
    use ntest::timeout;
    use uuid::Uuid;

    use crate::solution::{
        ProcessConfig, Raft, RaftMessage, RaftMessageContent, RaftMessageHeader, Shutdown,
    };
    use crate::{ActixSender, RamStorage};

    #[actix_rt::test]
    #[timeout(1500)]
    async fn single_process_transitions_to_leader() {
        // given
        let (tx, rx) = unbounded();
        let sender = ActixSender::default();
        let self_id = Uuid::new_v4();
        let raft = Raft::new(
            ProcessConfig {
                self_id,
                election_timeout: Duration::from_millis(200),
                processes_count: 1,
            },
            Box::new(RamStorage::default()),
            Box::new(sender.clone()),
        );
        let spy = RaftSpy::new(raft, tx).start();
        sender.insert(self_id, spy.recipient());

        // when
        actix::clock::delay_for(Duration::from_millis(700)).await;
        let msgs = extract_messages(&rx);

        // then
        assert_has_heartbeat_from_leader(self_id, &msgs);
    }

    fn assert_has_heartbeat_from_leader(expected_leader: Uuid, msgs: &Vec<RaftMessage>) {
        heartbeats_from_leader(msgs)
            .iter()
            .map(|t| t.1)
            .find(|leader_id| leader_id == &expected_leader)
            .expect("No heartbeat from expected leader!");
    }

    pub(crate) fn heartbeats_from_leader(
        msgs: &Vec<RaftMessage>,
    ) -> Vec<(RaftMessageHeader, Uuid)> {
        let mut res = Vec::new();
        for msg in msgs {
            match msg.content {
                RaftMessageContent::Heartbeat { leader_id } => {
                    res.push((msg.header, leader_id));
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
                let _ = tx.send(msg).await;
                raft.send(msg).await.unwrap()
            });
        }
    }
}
