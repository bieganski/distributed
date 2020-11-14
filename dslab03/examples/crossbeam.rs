use crossbeam_channel::{bounded, unbounded, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

fn create_crossbeam_channel() {
    // Creating a channel creates two objects, not one. This pattern is common in
    // Rust: it fits the strict ownership requirements. As names suggest, *Sender*
    // interface allows for insertion of messages – it can be cloned – and *Receiver*
    // allows for a blocking call which waits for messages. You may think of the
    // channel as of a ring buffer, for which insertion and reading are largely
    // independent. The objected are commonly named `tx` and `rx` in Rust.
    let (tx, rx): (Sender<u8>, Receiver<u8>) = unbounded();

    // Send does not block.
    tx.send(7).unwrap();

    // Recv blocks until a message arrives, or until all Sender objects of this
    // channel are dropped.
    let _ = rx.recv().unwrap();
}

fn channel_ends_dropped() {
    {
        let (tx, rx): (Sender<u8>, Receiver<u8>) = unbounded();
        drop(tx);

        // Dropping the last Sender reference makes the channel return errors on recv.
        // let a = rx.recv().unwrap();
        // println!("a: {}", a);
        // assert!(rx.recv().is_err());
    }
    {
        let (tx, rx) = unbounded();
        drop(rx);

        // Dropping the last Receiver reference makes the channel return errors on send.
        assert!(tx.send(0).is_err());
    }
}

fn timeout_receiving() {
    let (_tx, rx): (Sender<u8>, Receiver<u8>) = unbounded();

    // Waiting for a message only for a limited time.
    assert_eq!(
        Err(RecvTimeoutError::Timeout),
        rx.recv_timeout(Duration::from_millis(20))
    );
}

fn communication_between_threads() {
    let (tx, rx) = unbounded();
    let tx_clone = tx.clone();

    std::thread::spawn(move || {
        tx.send(42).unwrap();
    });
    assert_eq!(Ok(42), rx.recv());

    // Sending via cloned Sender.
    tx_clone.send(7).unwrap();
    assert_eq!(Ok(7), rx.recv());
}

fn bounded_channel() {
    // There are also channels which can hold at most the specified number of messages.
    let (tx, _rx) = bounded(1);
    tx.send(7).unwrap();

    // Calling another send blocks until the first message is received.
    // tx.send(7).unwrap();
}

use std::thread; // :sleep;
use std::time; 
fn zero_size_channel() {
    // A curiosity, but it is worth noting its existence.
    // Sending and receiving must happen at the same time for
    // a value to be passed.
    let (tx, rx) = bounded(0);
    std::thread::spawn(move || {
        // Blocks unitl recv is called.
        tx.send({}).unwrap();
        tx.send({}).unwrap();
    });
    let ten_millis = time::Duration::from_millis(3000);

    thread::sleep(ten_millis);
    rx.recv().unwrap();
}

fn main() {
    create_crossbeam_channel();
    channel_ends_dropped();
    timeout_receiving();
    communication_between_threads();
    bounded_channel();
    zero_size_channel();
}
