mod public_test;
mod solution;

use actix::{Addr, Recipient};
use async_channel::unbounded;

use crate::solution::{build_system, Circle, Read, Write, WriterProcess};

pub async fn read_value(reader: &Recipient<Read>) -> Circle {
    let (tx_reader_done, rx_reader_done) = unbounded();

    reader
        .try_send(Read {
            read_return_callback: Box::new(move |val| {
                tx_reader_done.try_send(val).unwrap();
            }),
        })
        .unwrap();

    rx_reader_done.recv().await.unwrap()
}

pub async fn write_value(writer: &Addr<WriterProcess>, value: Circle) {
    let (tx_writer_done, rx_writer_done) = unbounded();
    
    writer
        .try_send(Write {
            value,
            write_return_callback: Box::new(move || {
                tx_writer_done.try_send(()).unwrap();
            }),
        })
        .unwrap();

    let _ = rx_writer_done.recv().await.unwrap();
}

#[actix_rt::main]
async fn main() {
    env_logger::init();
    let (writer, readers) = build_system(3).await;
    assert_eq!(readers.len(), 2);

    write_value(&writer, Circle { x: 1, y: -1, r: 2 }).await;

    let val = read_value(&readers.get(0).unwrap().clone().recipient()).await;
    println!("Received value: {:?}", val);
    assert_eq!(val, Circle { x: 1, y: -1, r: 2 });
}
