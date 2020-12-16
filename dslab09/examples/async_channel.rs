use async_channel::unbounded;

async fn simple_channel() {
    let (tx, rx) = unbounded();

    tx.send(7).await.unwrap();
    // There is also sync send.
    tx.try_send(8).unwrap();

    assert_eq!(7, rx.recv().await.unwrap());
    assert_eq!(8, rx.recv().await.unwrap());
}

fn main() {
    // If you ever will need to simply resolve a future, you can use block_on.
    futures::executor::block_on(simple_channel());
}
