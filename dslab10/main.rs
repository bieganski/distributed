mod public_test;
mod solution;

use crate::solution::{
    CyberStore2047, Node, Product, ProductPriceQuery, ProductType, Transaction, TransactionMessage,
    TwoPhaseResult,
};
use actix::clock::{delay_for, Duration};
use actix::Actor;
use async_channel::unbounded;
use uuid::Uuid;

#[actix_rt::main]
async fn main() {
    let (transaction_done_tx, transaction_done_rx) = unbounded();
    let laptop_id = Uuid::new_v4();
    let initial_laptop_price = 150000;
    let products = vec![
        Product {
            identifier: laptop_id,
            pr_type: ProductType::Electronics,
            price: initial_laptop_price,
        },
        Product {
            identifier: Uuid::new_v4(),
            pr_type: ProductType::Books,
            price: 5000,
        },
        Product {
            identifier: Uuid::new_v4(),
            pr_type: ProductType::Toys,
            price: 1000,
        },
    ];
    let node = Node::new(products).start();
    let cyber_store = CyberStore2047::new(vec![node.clone().recipient()]).start();

    // Add one zloty to prices.
    let electronics_price_shift = 100;
    cyber_store
        .send(TransactionMessage {
            transaction: Transaction {
                pr_type: ProductType::Electronics,
                shift: electronics_price_shift,
            },
            completed_callback: Box::new(move |result| {
                actix::spawn(async move {
                    transaction_done_tx.send(result).await.unwrap();
                });
            }),
        })
        .await
        .unwrap();

    assert_eq!(Ok(TwoPhaseResult::Ok), transaction_done_rx.recv().await);
    delay_for(Duration::from_millis(100)).await;

    assert_eq!(
        initial_laptop_price + (electronics_price_shift as u64),
        node.send(ProductPriceQuery {
            product_ident: laptop_id,
        })
        .await
        .unwrap()
        .0
        .unwrap()
    );
    println!("System can execute a simple transaction!");
}
