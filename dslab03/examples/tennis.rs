use crossbeam_channel::{unbounded, Sender, select};

struct PingPong {
    name: &'static str,
    received_messages: usize,
    other: Option<PingPongRef>,
}

struct Ball {}

impl PingPong {
    fn new(name: &'static str) -> Self {
        PingPong {
            name,
            received_messages: 0,
            other: None,
        }
    }

    fn handler(&mut self, _msg: Ball) {
        self.received_messages += 1;
        println!(
            "The {} receives a Ball. It has received {} message(s) so far.",
            self.name, self.received_messages
        );

        // Let's send my Ball.
        // In a real implementation the message should be put to the executor's
        // queue, not directly to the other module's one.
        self.other.as_ref().unwrap().handle.send(Ball {}).unwrap();
    }
}

struct PingPongRef {
    handle: Sender<Ball>,
}

fn main() {
    let (tx1, rx1) = unbounded();
    let (tx2, rx2) = unbounded();

    let mut ping = PingPong::new("ping");
    let mut pong = PingPong::new("pong");

    tx1.send(Ball {}).unwrap();

    ping.other = Some(PingPongRef { handle: tx2 });
    pong.other = Some(PingPongRef { handle: tx1 });

    // There is no real executor's queue here. We just show the general idea
    // of how to decouple message sending from its receiving and handling.
    // The small assignment focuses on having just one executor's queue/channel.
    for _ in 0..5 {
        select! {
            recv(rx1) -> msg => ping.handler(msg.unwrap()),
            recv(rx2) -> msg => pong.handler(msg.unwrap()),
        };
    }
}
