#[cfg(test)]
pub(crate) mod tests {
    use actix::clock::Duration;
    use actix::Actor;
    use async_channel::unbounded;
    use ntest::timeout;
    use uuid::Uuid;

    use crate::raft::utils::{ActixSender, RamStorage};
    use crate::raft::{ProcessConfig, Raft};
    use crate::solution::{DistributedSet, IsPresent, SetOperation};

    #[actix_rt::test]
    #[timeout(1000)]
    async fn single_node_adds_to_set() {
        // given
        let ident_leader = Uuid::new_v4();
        let processes = vec![ident_leader];
        let sender = ActixSender::default();

        let (entries_tx, entries_rx) = unbounded();
        let raft_leader = Raft::new(
            entries_tx,
            ProcessConfig {
                self_id: ident_leader,
                election_timeout: Duration::from_millis(50),
                processes,
            },
            Box::new(RamStorage::default()),
            Box::new(sender.clone()),
        )
        .start();
        sender.insert(ident_leader, raft_leader.clone().recipient());
        // Raft leader election
        actix::clock::delay_for(Duration::from_millis(200)).await;
        let set = DistributedSet::new(raft_leader, entries_rx).start();

        // when
        set.send(SetOperation::Add(7_i64)).await.unwrap();
        // Sleep, as the assignment description suggests.
        actix::clock::delay_for(Duration::from_millis(100)).await;
        let is_7_present = set.send(IsPresent(7_i64)).await.unwrap();

        // then
        assert!(is_7_present);
    }
}
