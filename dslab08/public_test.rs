#[cfg(test)]
mod tests {
    use async_channel::unbounded;
    use ntest::timeout;

    use crate::solution::{build_system, Circle, Read};

    #[actix_rt::test]
    #[timeout(300)]
    async fn smoke_test_single_process() {
        let circle = Circle::default();
        let (writer, _readers) = build_system(1).await;
        let (tx_reader_done, rx_reader_done) = unbounded();

        writer
            .send(Read {
                read_return_callback: Box::new(move |val| {
                    tx_reader_done.try_send(val).unwrap();
                }),
            })
            .await
            .unwrap();

        assert_eq!(circle, rx_reader_done.recv().await.unwrap());
    }
}
