use actix::{Actor, AsyncContext, Context, Handler, Message, System};
use std::time::Duration;

/// This type will receive messages. There are a few traits to implement,
/// but apart from that you focus only on message handlers, not mundane details.
struct Printer {}

struct Print {
    msg: &'static str,
}

/// Message is a trait any message to be sent by actix has to implement.
impl Message for Print {
    /// The type of value that this message will resolved with if it is successful, usually `()`.
    type Result = ();
}

/// Main trait to implement is Actor. The Context specifies what context type
/// is available to your actor - most often `Context<Self>`. The context is for
/// technical details with respect to running actors - primary purpose is to request
/// stopping of an actor from message handler. So you probably would use other Context
/// types only when implementing some framework on top of actix.
impl Actor for Printer {
    type Context = Context<Self>;
}

/// This is how to implement message handlers. The concept of introducing a special trait
/// ought to be familiar - it was introduced in description of the first assignment.
/// The associated Result type is what is the reply to the message, most often
/// you will want `()` here.
impl Handler<Print> for Printer {
    type Result = ();

    fn handle(&mut self, msg: Print, _ctx: &mut Self::Context) -> Self::Result {
        println!("Printing: {}", msg.msg);
        // System::current() is a reference to system the actor is running on.
        System::current().stop_with_code(0);
    }
}

fn simple_send() {
    // This creates a new system of actors in the current thread.
    let system = System::new("sample-system");

    // Inserting actor to the current system.
    let printer = Printer {}.start();
    // Sending a message returns a future
    let r = printer.send(Print {
        msg: "Hello world!",
    });

    // This is a technical detail - we want a future to complete, but
    // also to start actor system - so a new thread is created, which waits on future.
    std::thread::spawn(move || {
        futures::executor::block_on(r).unwrap();
    });

    // Run system, this function blocks current thread.
    // Message delivery stops the system, so the function returns quickly.
    system.run().unwrap();
}

struct Timer {
    dur: Duration,
}

impl Actor for Timer {
    type Context = Context<Self>;

    // Stop system after `self.dur` seconds.
    fn started(&mut self, ctx: &mut Context<Self>) {
        // Default context provide a lot of methods - it can schedule
        // delivery of messages. The best information about Context will
        // always be on its page on actix docs - but keep in mind that
        // Context is the place to go with everything outside of processing
        // messages.
        ctx.run_later(self.dur, |_act, _ctx| {
            println!("Timer done");
            System::current().stop_with_code(0);
        });
    }
}

fn sync_actor_timer() {
    // Explicit creation of an actix system.
    let sys = System::new("test");

    Timer {
        dur: Duration::new(1, 0),
    }
    .start();

    // After one second from starting, the system shuts down.
    sys.run().unwrap();
}

fn main() {
    simple_send();
    sync_actor_timer();
}
