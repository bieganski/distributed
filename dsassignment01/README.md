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

## Reliable broadcast implementation details
System is in `correct state` if at least 50% processes is working properly.
Reliable broadcast guarantees, that providing that system is in `correct state`, it wil deliver broadcasted
message to each proper process.



