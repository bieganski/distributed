// WARNING: Do not modify definitions of public types or function names in this
// file – your solution will be tested automatically! Implement all missing parts.

use crossbeam_channel::{unbounded, Receiver, Sender};
use std::thread;
use rand::Rng;
use std::collections::HashMap;

type Num = u64;
type Ident = u128;

#[derive(Debug)]
pub(crate) struct FibonacciModule {
    /// Currently hold number from the sequence.
    num: Num,
    /// Index of the required Fibonacci number (the `n`).
    limit: usize,
    /// Identifier of the module.
    id: Ident,
    /// Identifier of the other module.
    other: Option<Ident>,
    /// Queue for outgoing messages.
    queue: Sender<EngineMessage>,
}

impl FibonacciModule {
    /// Creates the module and registers it in the engine.
    pub(crate) fn create(
        initial_number: Num,
        limit: usize,
        queue: Sender<EngineMessage>,
    ) -> Ident {
        // For the sake of simplicity, let us generate here a random identifier.
        let id = rand::thread_rng().gen();
        
        let module = FibonacciModule {
            num :initial_number, 
            id : id,
            other : None,
            limit : limit,
            queue : queue.clone() // how to do it without cloning?
        };

        queue.send(EngineMessage::RegisterModule(module));

        id
    }

    /// Handles the step-message from the other module.
    ///
    /// Here the next number of the Fibonacci sequence is calculated.
    pub(crate) fn message(&mut self, idx: usize, num: Num) {
        if idx >= self.limit {
            // The calculation is done.
            self.queue.send(EngineMessage::Done);
        }

        self.num += num;

        self.queue.send(EngineMessage::Message{id: self.other.unwrap(), idx: idx + 1, num: self.num});

        // Put the following print statement after performing an update of `self.num`.
        println!("Inside {}, value: {}", self.id, self.num);
    }

    /// Handles the init-message.
    ///
    /// The module finishes its initialization and initiates the calculation
    /// if it is the first to go.
    pub(crate) fn init(&mut self, other: Ident) {
        self.other = Some(other);
        
        if self.num == 0 {
            self.queue.send(EngineMessage::Init{id : self.id, other : other});
            self.queue.send(EngineMessage::Message{id : other, idx : 1, num : self.num});
        }
    }
}

/// Messages sent to/from the modules.
///
/// The `id` field denotes which module should receive the message.
pub(crate) enum EngineMessage {
    /// Registers the module in the engine.
    ///
    /// Note that this is a struct without named fields: a tuple struct.
    RegisterModule(FibonacciModule),

    /// Finalizes module initialization and initiates the calculations.
    Init { id: Ident, other: Ident },

    /// Initiates the next step of the calculations.
    ///
    /// `idx` is the index in the sequence.
    Message { id: Ident, idx: usize, num: Num },

    /// Indicates the end of calculations.
    Done,
}

/// Calculates `n`-th Fibonacci number.
pub(crate) fn fib(n: usize) {
    // Create two modules here.
    let (tx, rx): (Sender<EngineMessage>, Receiver<EngineMessage>) = unbounded();
    let fib1_id = FibonacciModule::create(0, n, tx.clone());
    let fib2_id = FibonacciModule::create(1, n, tx.clone());

    // Tests will be rerun in case of this...
    assert_ne!(fib1_id, fib2_id);

    // Initialize modules here.
    // TODO
    //unimplemented!();

    // Start the executor.
    let executor = thread::spawn(move || {
        let mut map = HashMap::new();
        let f1 : Option<FibonacciModule> = None;
        let f2 : Option<FibonacciModule> = None;
        let state = 0;
        while let Ok(msg) = rx.recv() {
            match msg {
                EngineMessage::RegisterModule(mut fib) => {
                        fib.init(if fib.id == fib1_id { fib2_id.clone() } else { fib1_id.clone() });
                        map.insert(fib.id, fib);
                    },
                EngineMessage::Init{id, other} => {
                    // TODO
                    //
                    // what should I do here?
                    //
                    // TODO
                },
                EngineMessage::Message{id, idx, num} => {
                    let mut fib = map.remove(&id).unwrap();
                    fib.message(idx, num);
                    map.insert(fib.id, fib);
                },
                EngineMessage::Done => {
                    return;
                }
            }
        }
    });

    executor.join().unwrap();

    // Sample result of `cargo run`:
    //
    // Inside 109630159281952332903990402733657913010, value: 1
    // Inside 90776690821287183793072922585987852479, value: 2
    // Inside 109630159281952332903990402733657913010, value: 3
    // Inside 90776690821287183793072922585987852479, value: 5
    // Inside 109630159281952332903990402733657913010, value: 8
    // Inside 90776690821287183793072922585987852479, value: 13
    // Inside 109630159281952332903990402733657913010, value: 21
    // Inside 90776690821287183793072922585987852479, value: 34
    // Inside 109630159281952332903990402733657913010, value: 55
    //
    // The identifiers are random, of course.

}
