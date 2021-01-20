#[cfg(test)]
mod tests {
    use actix::Actor;
    use async_channel::unbounded;
    use ntest::timeout;

    use crate::solution::{
        CyberStore2047, Node, Product, ProductType, Transaction, TransactionMessage, TwoPhaseResult,
    };
    use uuid::Uuid;

    #[actix_rt::test]
    #[timeout(300)]
    async fn transaction_with_two_nodes_completes() {
        // given
        let (transaction_done_tx, transaction_done_rx) = unbounded();
        let products = vec![Product {
            identifier: Uuid::new_v4(),
            pr_type: ProductType::Electronics,
            price: 180,
        }];
        let node0 = Node::new(products.clone()).start();
        let node1 = Node::new(products).start();
        let cyber_store = CyberStore2047::new(vec![node0.recipient(), node1.recipient()]).start();

        // when
        cyber_store
            .send(TransactionMessage {
                transaction: Transaction {
                    pr_type: ProductType::Electronics,
                    shift: -50,
                },
                completed_callback: Box::new(move |result| {
                    actix::spawn(async move {
                        transaction_done_tx.send(result).await.unwrap();
                    });
                }),
            })
            .await
            .unwrap();

        // then
        assert_eq!(
            TwoPhaseResult::Ok,
            transaction_done_rx.recv().await.unwrap()
        );
    }
}
