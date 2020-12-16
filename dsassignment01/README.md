<a href = "https://www.rust-lang.org/">
<img width = "90%" height = "auto" src = "https://img.shields.io/badge/Rust-Programming%20Language-black?style=flat&logo=rust" alt = "The Rust Programming Language">
</a>

# Executor System Library

The library (`/solution/lib.rs`) allows user to create set of different-type Modules, communicating with each other. All modules registerd in `System` instance, together with `Executor thread` implement `shared-nothing` architecture.
Each module may be responsible for one specific task, and only communicate with another modules via Messages.
For each message type `T` and module `M` it's up to you to provide `Handler <T> for M`.

Multiple nodes may be used - each instance of `System` may run on single machine. It's up to you to provice implementation of `dyn PlainSender` trait, with `send_to(Uuid)` method - it may be either TCP or UDP IP packet, or `crossbeam_channel` `(Sender, Receiver)`.

### Example

```rust
struct PingPong {
    other: Option<ModuleRef<PingPong>>,
    received_msgs: u32,
    first: bool,
    name: &'static str,
}
```

```rust
struct Ball {}
struct Init {
    target: ModuleRef<PingPong>,
}
```

```rust
impl Handler<Init> for PingPong {
    fn handle(&mut self, msg: Init) {
        self.other = Some(msg.target);
        if self.first {
            self.other.as_ref().unwrap().send(Ball {});
        }
    }
}
```

```rust
impl Handler<Ball> for PingPong {
    fn handle(&mut self, _msg: Ball) {
        println!("In {}: received {}\n", self.name, self.received_msgs);

        self.received_msgs += 1;
        if self.received_msgs < 5 {
            self.other.as_ref().unwrap().send(Ball {})
        }
    }
}
```

## Output
```
In Pong: received 0
In Ping: received 0
In Pong: received 1
In Ping: received 1
In Pong: received 2
...
```


## Build & Run

```shell
cargo build
cargo run
```

## Tests

```shell
cd public-tests
cargo test
# or
cargo test --test <test_name>
```

# Reliable Broadcast Module

User can provide his own modules, but we provide one specific module - `Reliable broadcast module`.
Function `setup_system(..)` returns `ModuleRef<ReliableBroadcastModule>`.

`ReliableBroadcast` module is able to send foolproof broadcast messages to all defined `Uuids` (may be different nodes, providing correct `PlainSender` implementation). You can use it the following way:
```rust
let msg_content : Vec <u8> = ...;
let bcast_ref = setup_system(...);
bcast_ref.send(SystemMessageContent{msg: msg_content});
```

# Implementation details

### Logged Majority Ack uniform reliable broadcast:
System is in `correct state` if at least 50% processes is working properly.
Reliable broadcast guarantees, that providing that system is in `correct state`, it wil deliver broadcasted
message to each proper process. It takes advantage of `StubbornBroadcast`, that stubbornly sends messages to all processes, until receive acknowledgment from each.
### Pseudocode:
```
Implements:
    LoggedUniformReliableBroadcast, instance lurb.

Uses:
    StubEbornBestEffortBroadcast, instance sbeb. Described below.

upon event < lurb, Init > do
    delivered := empty;
    pending := empty;
    forall m do ack[m] := empty;

upon event < lurb, Recovery > do
    retrieve(pending, delivered);
    forall (s, m) `in` pending do
        trigger < sbeb, Broadcast | [DATA, s, m];

upon event < lurb, Broadcast | m > do
    pending := pending + {(self, m)};
    store(pending);
    trigger < sbeb, Broadcast | [DATA, self, m] >;

upon event < sbeb, Deliver | p, [DATA, s, m] > do
    if (s, m) `not in` pending then
        pending := pending + {(s, m)}
        store(pending);
        trigger < sbeb, Broadcast | [DATA, s, m] >;
    if p `not in` ack[m] then
        ack[m] := ack[m] + {p};
        if #(ack[m]) > N/2 `and` (s, m) `not in` delivered then
            delivered := delivered + {(s, m)};
            store(delivered);
            trigger < lurb, Deliver | delivered >;

Implements:
    StubbornBestEffortBroadcast, instance sbeb.

Uses:
    StubbornPointToPointLinks, instance sl

upon event < sbeb Broadcast | m > do
    forall q `in` all_processes
        trigger < sl, Send | q, m >;

upon event < sl, Deliver | p, m > do
    trigger < sbeb, Deliver | p, m >;
```

`store` and `retrieve` functions operate on `Stable storage` (disk implementation of (key, value) `get`, `put` methods)

### Multiple-type message
Let's consider possible implementation of `executor thread` of `Ping-Pong` example:
```rust
let executor = thread::spawn(move || {
        let mut map = HashMap::new();
        while let Ok(msg) = rx.recv() {
            match msg {
                Init{who} => {
                        map.get_mut(who).initialize();
                        ...
                    },
                Ball => {
                    // COMPILE ERROR! 'msg' type is Init or Ball, cannot be both!!!
                }
            }
        }
    });
```
Implementation above doesn't allow for messages of multiple types. Instead, we achive it using templates and closures. Keep in mind, that `pub struct System {...}` is not parametrized with any type `T`: thus it cannot keep any generic type. However, it may keep `HashMap<ModId, Box<dyn WorkerType + Send>>`, and only be provided with closures to run. Executor is inifnite loop is pooling fixed-type, meta receiver that points to module-specific queue with lambda to be run.
```rust
match executor_meta_rx.recv().unwrap() {
    NewModule(_) => {
        ...
    },
    RemoveModule(id) => {
        ...
    },
    ExecuteModule(id) => {
        let mut map = workers_cloned_executor.lock().unwrap();
        let mut w = map.get_mut(&id).unwrap();
        let worker : &mut Box<dyn WorkerType + Send> = &mut w;
        worker.execute();
    },
}
```

```rust
struct ModuleRefData<T: 'static> {
        meta: Sender<WorkerMsg>, // informs executor about pending messages
        worker_buffer: Sender<MessageLambda<T>>,
        id: ModId,
}
pub struct Worker<T: 'static> {
    receiver: Receiver<MessageLambda<T>>,
    module: T,
}

pub trait WorkerType {
    fn execute(&mut self);
}

impl<T> WorkerType for Worker<T> {
    fn execute(&mut self) {
        let f = self.receiver.recv().unwrap();
        f(&mut self.module);
    }
}

impl<T> ModuleRef<T> {
    pub fn send<M: Message>(&self, msg: M)
    where
        T: Handler<M>,
    {
        let message = move |module: &mut T| {
            module.handle(msg);
        };
        self.data.worker_buffer.send(Box::new(message)).unwrap();
        self.data.meta.send(ExecuteModule{0: self.data.id}).unwrap();
    }
}
```

### Possible memory leaks
Dropping last reference to `ModuleRef` may imply `System` memory leak - to handle that, `Executor` is able to handle message `RemoveModule(id)`, that frees resources. That message must be sent in some way in ModuleRef destructor; however, it must be sent only once; dropping the last reference. To achieve that, we propose struture below:

```rust
pub struct ModuleRef<T: 'static> {
    data: Arc<ModuleRefData<T>>,
}

impl<T> Drop for ModuleRefData<T> {
    fn drop(&mut self) {
        self.meta.send(RemoveModule{0: self.id}).unwrap();
    }
}
```
This is neat way to outsource reference counting to `Arc` structure.




