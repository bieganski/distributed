# Distributed Systems course at MIMUW (in Rust)

### Content

* **dslab01** - getting familiar with Rust, function calculating n-th Fibonacci number
* **dslab02** - concurency in Rust (mutex, conditional variable). threadpool implementation 
* **dslab03** - Event-driven shared-nothing architecture (message passing architecture)
* **dslab04** - secure layer of Ethernet communication (Client + Server):
	* TLS
	* RSA
	* AES
	* MAC
* **dslab05** - generic list implementation
* **dslab06** - simple block read/write device Linux kernel driver
* **dslab07** - Actix and Tokio examples 
* **dslab08** - Asynchronous implementation of (1, N) distributed register (Actix)
* **dslab09** - Atomic Distributed Device
* **dslab10** - Distributed commit
* **dslab11** - Consensus
* **dslab12** - Log replication (Raft)

Large libraries:
* [Executor](tree/main/dsassignment1/) - Implementation of message-only, shared-nothing architecture (executor with modules), with module that implements Uniform Logged Reliable Broadcast Algorithm
* [Atomic register](tree/main/dsassignment2/) - Distributed, (N,N) atomic register implementation over TCP (custom protocoli specs), highly concurrent and crash-resistant (state persistency)


### Build

```shell
cd /dslab<NUM>
cargo build
carg run
```

### Build examples

```shell
cd dslab<NUM>
cargo run --example <example name>
```
