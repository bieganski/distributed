use actix::prelude::*;

struct PowerActor {
    base: u32,
}

impl Actor for PowerActor {
    type Context = Context<Self>;
}

struct Num {
    n: u8,
}

struct Print {
    num: u128,
}

impl Message for Num {
    // Response type for message - reply type for handler.
    type Result = u128;
}

impl Message for Print {
    type Result = ();
}

impl Handler<Num> for PowerActor {
    // In handler, return type must be wrapped for technical reasons
    // (internal actix layout of traits). Message result can be used then as a future.
    type Result = MessageResult<Num>;

    fn handle(&mut self, msg: Num, _ctx: &mut Self::Context) -> Self::Result {
        let res = self.base as u128;
        // Message::Result is inserted into MessageResult. Note that is is not
        // as inefficient as it might look - only the type checking needs a type
        // which has a single field, generated machine code needs no notion of it.
        MessageResult(res.pow(msg.n.into()))
    }
}

impl Handler<Print> for PowerActor {
    type Result = ();

    fn handle(&mut self, msg: Print, _: &mut Self::Context) -> Self::Result {
        println!("Power: {}", msg.num);
    }
}

impl From<u8> for Num {
    fn from(n: u8) -> Self {
        Num { n }
    }
}
use std::{thread, time};

#[actix_rt::main]
async fn async_actor_example() {
    let ten_millis = time::Duration::from_millis(3000);
    thread::sleep(ten_millis);
    let addr = PowerActor { base: 2 }.start();
    // Waiting for returned result - that simple.
    // Think how problematic it would be in a synchronous function -
    // actor system would be completely decoupled from your thread of execution.
    let result = addr.send(Num::from(10)).await.unwrap();
    addr.send(Print { num: result }).await.unwrap();
}

fn main() {
    async_actor_example();
    println!("Done");
}
