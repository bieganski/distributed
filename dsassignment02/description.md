# Distributed Systems assignment 2

## Atomic Disc Drive

Second assignment, about distributed registers and using them to provide durable storage.

# Background

We would like you to implement a distributed block device, called *Atomic Disc Drive*. It stores
data in a distributed register, and can be used like any other block device.

Distributed register consists of multiple processes running in user space, possibly on many
different machines. The block device driver connects to them using TCP. Also the processes
themselves communicate using TCP. Processes can crash and recover at any time. Number of
processes is fixed before the system is run, and every process will have its own directory
on the disc. By process in this document we will mean a single entity from a distributed
register, not an operating system process.

Every sector of the block device is a separate atomic value stored in the distributed system,
meaning the system supports a set of of atomic values, called also *registers*. Sectors
have 4096 bytes.

Originally, we thought that implementing a driver would be a part of this assignment, but after
the first assignment we decided it would be too much. We hand out to you our simple reference
driver (`driver/` directory), which you can use to test your solution. It is described
in in `linux_driver.md`, this file will focus on the distributed system.

## Solution interface

Your implementation will take the form of cargo library crate. 

You have to implement the `run_register_process` method from `solution/src/lib.rs`. This is
the main focus of the assignment, and what will be the focus of testing. This method will
be used by a simple wrapper—program to run your solution. Note that solution must be asynchronous,
as efficiency will be a focus of this assignment. Your solution will be run using tokio
runtime engine.

As noted before, processes communicate using TCP. Relevant information for communication
is passed in *Configuration* (`solution/src/domain.rs`) as an argument.

### Technical build details

You can use only crates listed in `Cargo.toml` in the `solution/` for main dependencies,
and anything you want for `[dev-dependencies]` section, as this section will be ignored by
tests. You cannot  use `[build-dependencies]`, and you can specify any number of binaries in your
`Cargo.toml`—those also will be ignored by tests. If you need any other crate, ask
on Moodle for permission. If a permit is granted, every student is free to use it.

Using asynchronous libraries makes it easy to scale your solution to the number
of available cores, and wait for completion of hundreds of concurrent IO tasks.
This is necessary to reach acceptable performance.

Because crashes are to be expected, in *Configuration* there is also a directory
for exclusive use by your process.

Now we move on to general overview of the assignment, with more details in the section
at the end of this document.

## Client commands

We begin with description of the interface of your distributed system as seen by clients.
Every process of the system can be contacted by a client. Clients connect using TCP, and
can send *READ* and *WRITE* commands. Your process must issue replies after a particular
command was safely completed by the distributed system. Semantics of commands is mostly
self-explanatory, so we focus on stating their format. Numbers are always in network byte order.

*READ* and *WRITE* have the following format:

    0       7 8     15 16    23 24    31 32    39 40    47 48    55 56    64
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |             Magic number          |       Padding            |   Msg  |
    | 0x61     0x74      0x64     0x64  |                          |   Type |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Request number                              |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Sector index                                |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Command content ...
    |
    +--------+--------+--------...
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+

*READ* operation type is 0x01, and 0x02 stands for *WRITE*. Client can use request number for
internal identification of messages.

*WRITE* has content with 4096 bytes with bytes to be written. *READ* has no content.
HMAC tag is `hmac(sh256)` tag of the entire message (from magic number to the end of content).

After the system completes any of those two operations, it must reply with response
in the following format:

    0       7 8     15 16    23 24    31 32    39 40    47 48    55 56    64
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |             Magic number          |     Padding     | Status |  Msg   |
    | 0x61     0x74      0x64     0x64  |                 |  Code  |  Type  |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Request number                              |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Response content ...
    |
    +--------+--------+--------...
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+

Again HMAC tag is `hmac(sha256)` of the entire response. Status codes will be explained shortly.
Response content for successful *WRITE* is empty, while for successful *READ* it has 4096
bytes read from the system. If a command has failed for any reason, it has empty
response content. Response message type is always 0x40 added to original message type,
so READ* response type is 0x41, while *WRITE* has 0x42.

Command types starting with `0x80` are free to use for you-you can use them for debugging
or for any other purpose. Some additional commands will be defined later on, they will be
specific to the register algorithm.

Requests with invalid HMAC must be discarded with appropriate status code returned.
A HMAC key for client commands and responses is in the *Configuration.hmac_client_key*
(`solution/src/domain.rs`) and is shared with client externally by tests.
This is not the cleanest cryptographic setup, but this scheme is easy to implement.

### Response status codes

Possible status codes are listed in *StatusCode* `solution/src/domain.rs`.
Doc strings in this file explain when each code is expected. Status codes are to be
represented as single consecutive bytes, starting with 0x00 as *Ok*.

# Distributed register

Here we describe what algorithm you will use for the assignment to provide the register.
This will be the bedrock of your distributed system.

## (N, N)-AtomicRegister

There is a fixed number of processes, they know about each other. Crashes of individual
processes can happen. The `(N, N)-AtomicRegister` takes its name from the fact that
every process can initiate both read and write operation. For such register, we assume
that at least majority of the processes are working correctly for it to be able to
make progress on operations. Here we provide the core algorithm, based on the
*Reliable and Secure Distributed Programming* by C. Cachin, R. Guerraoui, L. Rodrigues and
modified to suit crash-recovery model:

```text
Implements:
    (N,N)-AtomicRegister instance nnar.

Uses:
    StubbornBestEffortBroadcast, instance sbeb;
    StubbornLinks, instance sl;

upon event < nnar, Init > do
    (ts, wr, val) := (0, 0, _);
    rid:= 0;
    readlist := [ _ ] `of length` N;
    acklist := [ _ ] `of length` N;
    reading := FALSE;
    writing := FALSE;
    writeval := _;
    readval := _;
    store(wr, ts, val, rid, writing, writeval);

upon event < nnar, Recovery > do
    retrieve(wr, ts, val, rid, writing, writeval);
    readlist := [ _ ] `of length` N;
    acklist := [ _ ]  `of length` N;
    reading := FALSE;
    readval := _;
    if writing = TRUE then
        trigger < sbeb, Broadcast | [READ_PROC, rid] >;

upon event < nnar, Read > do
    rid := rid + 1;
    store(rid);
    readlist := [ _ ] `of length` N;
    acklist := [ _ ] `of length` N;
    reading := TRUE;
    trigger < sbeb, Broadcast | [READ_PROC, rid] >;

upon event < sbeb, Deliver | p [READ_PROC, r] > do
    trigger < pl, Send | p, [VALUE, r, ts, wr, val] >;

upon event <sl, Deliver | q, [VALUE, r, ts', wr', v'] > such that r == rid do
    readlist[q] := (ts', wr', v');
    if #(readlist) > N / 2 and (reading or writing) then
        (maxts, rr, readval) := highest(readlist);
        readlist := [ _ ] `of length` N;
        acklist := [ _ ] `of length` N;
        if reading = TRUE then
            trigger < sbeb, Broadcast | [WRITE_PROC, rid, maxts, rr, readval] >;
        else
            trigger < sbeb, Broadcast | [WRITE_PROC, rid, maxts + 1, rank(self), writeval] >;

upon event < nnar, Write | v > do
    rid := rid + 1;
    writeval := v;
    acklist := [ _ ] `of length` N;
    readlist := [ _ ] `of length` N;
    writing := TRUE;
    store(wr, ts, rid, writeval, writing);
    trigger < sbeb, Broadcast | [READ_PROC, rid] >;

upon event < sbeb, Deliver | p, [WRITE_PROC, r, ts', wr', v'] > do
    if (ts', wr') > (ts, wr) then
        (ts, wr, val) := (ts', wr', v');
        store(ts, wr, val);
    trigger < pl, Send | p, [ACK, r] >;

upon event < pl, Deliver | q, [ACK, r] > such that r == rid do
    acklist[q] := Ack;
    if #(acklist) > N / 2 and (reading or writing) then
        acklist := [ _ ] `of length` N;
        if reading = TRUE then
            reading := FALSE;
            trigger < nnar, ReadReturn | readval >;
        else
            writing := FALSE;
            store(writing);
            trigger < nnar, WriteReturn >;
```

For `rank(*)` you can use *Configuration.public.self_rank* (`solution/domain.rs` -
a single byte), while `highest(*)` returns the largest value ordered by `(timestamp, rank)`.

Details of the methods were explained during the lectures. Here we specify the binary interface of
commands used in the algorithm, as your solution is expected to operate with processes running
a reference solution. Your solution must provide atomic writes to sectors of data.

Your solution will not be receiving special *Recovery* or *Init* events, every time
it starts it must try to recover from stable storage—just during the initial run, there will
be nothing to recover. Expect crashes at any point—we expect your solution to work
despite them. The algorithm presented above is only pseudocode, so
we suggest understanding ideas behind it.

## Algorithm guarantees

Here we will discuss why such algorithm is suitable to the task of providing a block device.

Distributed systems do not have a common clock-events can happen at different speeds
and in various orders in each and every process. However, the atomic register enforces
constraints between events on processes, which make it possible to put every read and write
operation on a single timeline, and mark start and end of every operation. Every read
returns the last value written. If an operation `o` has happened before operation
`o'` when the system was processing messages, then `o` must appear before `o'` in such squeezing
into common timeline. This is called a *linearization*.

To summarize it, from a single client's perspective (a Linux driver), there is a single
progression of read and write events. It cannot distinguish it from a single entity which
performs read and write operations—like a disc.

## Algorithm specific commands interface

There is a common header for internal (process to process) messages:

    0       7 8     15 16    23 24    31 32    39 40    47 48    55 56    64
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |             Magic number          |       Padding   | Process|  Msg   |
    | 0x61     0x74      0x64     0x64  |                 |  Rank  |  Type  |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           UUID                                        |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           UUID                                        |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Read identifier                             |
    |                        of register operation                          |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Sector                                      |
    |                           index                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Message content ...
    |
    +--------+--------+--------...
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+

Those internal commands must be signed with different key then client's commands. It can be
found in *Configuration.hmac_system_key* (`solution/src/domain.rs`). Process rank
is rank of the process which sends a command. Now we will describe additional content
for every message.

### *READ_PROC*

Type 0x03, no content.

### *VALUE*

Type 0x04, content:

    0       7 8     15 16    23 24    31 32    39 40    47 48    55 56    64
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Timestamp                                   |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                 Padding                                      | Value  |
    |                                                              | wr     |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Sector data ...
    |
    +--------+--------+--------...

`Value wr` is the write rank of the process which delivered last write.
Sector contains 4096 bytes.

### *WRITE_PROC*

Type 0x05, content:

    0       7 8     15 16    23 24    31 32    39 40    47 48    55 56    64
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Timestamp                                   |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                 Padding                                      | Value  |
    |                                                              | wr     |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           Sector data ...
    |
    +--------+--------+--------...

Sector data has 4096 bytes to be written, while `value wr` is the write rank of
the process which delivered last write.

### *ACK*

Type 0x06, no content.

### System command response

Acknowledge responses for system commands are expected. They must contain the UUID
of message that is being acknowledged, and be signed with *Configuration.hmac_system_key*:

    0       7 8     15 16    23 24    31 32    39 40    47 48    55 56    64
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |             Magic number          | Padding| Status |Process |  Msg   |
    | 0x61     0x74      0x64     0x64  |        |  Code  |  Rank  |  Type  |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           UUID                                        |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           UUID                                        |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+
    |                           HMAC tag                                    |
    |                                                                       |
    +--------+--------+--------+--------+--------+--------+--------+--------+

Source rank stands for rank of the process which has originally issued the message
and created its UUID. Response message type is always 0x40 added to original message type,
so e.g. 0x46 for *ACK*.

## Performance

Register algorithm as described in the previous section can only make progress on one
read or write at a time. However, note this restriction applies only to a single value. This
means your solution can run multiple instances of atomic register logic, each making progress
on a different sector. We expect that anything above a bare-minimum solution will provide
such kind of concurrency and will be able to process many distinct sectors at once.

We expect that your single—process—system will be able to process on average 25 sectors
per second per logical core (with SSD drive). Of course this depends on actual hardware,
but it also is a very (very) soft requirement.

There is a limit of open file descriptors, 1024. We suggest that you solution utilizes
it for maximum concurrency.

Assume that local filesystem stores data in blocks of 4096 bytes, the same size as of sectors.
When sending messages to the same process which is issuing them, you should strive to skip TCP,
serialization and deserialization phases.

# Internal interfaces

We would like to be able to test your implementation of the atomic register in a more
fine-grained manner then heavy-weight tests with multiple operating system processes.
Having smaller unit tests allows also rapid grading of solutions. We allocate some points
to them, so in principle it should be easier to get more points when there is some
error in solution (as small errors are not likely to affect more then one unit test).

Having said that, what we deeply care about is the `run_register_process`. If your implementation
of this method is perfect, then you will receive maximum number of points. However, if it
is not, we want to have methods running smaller parts of your solution, so we will be able
to grade it in reasonable time.

Architecture is split into a few straightforward parts. However, almost every interface is
asynchronous—running the register will results in multiple IO tasks, and cooperative
multitasking seems to be a good way of expressing such software.

This section explains internal interfaces of the system. Your solution has to provide
implementations for them. Every following title is a trait defined
in `solution/src/lib.rs`.

We prepared an diagram (`atdd.svg`) of how our solution looks like. Every process of
the system is wrapped in a Linux process. Tokio is used as the executor system. Driver
sends commands to processes over TCP. Processes communicate with one another using system
messages, and then complete commands and return responses over TCP back to the driver.
Every process of the distributed system has multiple copies of the atomic register code,
to support concurrent writes/reads on distinct sectors. *RegisterClient* is shared between
all copies of atomic register code. Sectors management is also shared between those copies.

Now we move on to specific interfaces.

## *AtomicRegister*

*AtomicRegister* provides functionality required of the process of atomic register. It
is created with `build_atomic_register` (`solution/src/lib.rs`). Upon
construction, it must attempt to recover from its stable storage. All methods must map
to their semantics described in the section about atomic register, and must follow
the algorithm presented there. Your implementation of *AtomicRegister* can treat
*RegisterClient* as a `StubbornLink`, just like the main algorithm requires.

`build_atomic_register` returns also possibly a command that was in progress and
interrupted by a crash. We also thought it might be convenient to separate storage for atomic
register metadata (*StableStorage*) from the main storage for sectors data, at least on
the conceptual level. Note that you are given a directory, which means you can create as
many subdirectories as you wish for various purposes. The split is there for efficiency
reasons, as main storage for sectors is not expected to be a general purpose storage.
This means that when recovering from *StableStorage*, you should not restart the operation
if returned by `build_atomic_register`, as an exception to the general rule of booting
from *StableStorage*.

## Filesystem directory management, *SectorsManager*

Every process in your system will store its own copy of data, as represented by *SectorsManager*
trait (`solution/src/lib.rs`). You have a directory on the filesystem to keep
data (remember, you can create subdirectories, also for other purposes then *SectorsManager*),
provided via *Configuration.public.storage_dir* (`solution/src/domain.rs`). Every sector
of data is stored along with basic information—logical timestamp and rank of the process.
If a sector was never written, we assume that both timestamp and rank are zero. Sectors
are numbered from 0 inclusive to *Configuration.public.max_sector* exclusive
(`solution/src/domain.rs`).

If a sector was never written, we assume that its contents are 4096 zero bytes. Its logical
timestamp and rank are also zero.

We will accept at most 10% overhead of used filesystem space when compared with the size
of sectors which were stored. That is, if *n* distinct sectors were stored, we expect
total directory size not to exceed `1.1 * n * 4096` bytes. Of course, we expect this
to take effect for *n* greater or equal one thousand.

Apart from that, we do not care what scheme you will implement. Just remember to provide
atomic operations. No caching in your code is necessary.

Function `build_sectors_manager` (`solution/src/lib.rs`) must provide your implementation
for us to perform unit tests. You may assume that unit tests will not perform
concurrent operations on the same sector, even though the trait is marked as *Sync*.

## Serialization

We would like you to provide two methods, `deserialize_register_command` and
`serialize_register_command`. They respectively convert bytes to *RegisterCommand*
object and in the other direction. They need to provide schema mentioned previously in this text,
but without hmac tag—as appending or validating a hmac tag is external to a *RegisterCommand*.

Serialization must complete successfully when there are no errors when performing
writes to *Write* reference. If there are some errors, report them back.

Deserialization can report any error you like in case of invalid message type or for
invalid magic number. Note that you can add additional `derive` trait markers to public
definitions, as they only expand public interface.

As always, in principle we care only about *run_register_process*, but this is yet another
hook into your solution for unit tests.

## Stable storage for metadata

Your solution will store a limited amount of additional metadata apart from sectors in form
of durable key-value store. This will not be tested separately, and it is here for the sake
of tests of atomic register implementation. For details refer to the algorithm description.

## *RegisterClient*

Struct providing this trait is passed to your *AtomicRegister* implementation. The sole
purpose of this interface is to enable unit testing.

However, your solution will require a decent TCP client, as processes communicate
with one another. We do not provide hard requirements here, but note that your solution
is expected to perform under load. That is, we expect it to be efficient.

When sending message to self, it would increase performance if it would not be done over
TCP, but in some more direct manner.

# System technical overview

Now some final technical remarks about the system. This section is concerned with
`run_register_process` function (`solution/src/lib.rs`). This is an asynchronous
function, which must run your register and await for commands over TCP. Your system
will be run using *tokio* executor, which provides asynchronous APIs for the operating
system and facilities for spawning async tasks. Some choice had to be made, because Rust
does not have portable async OS interface yet.

## Logging

Now we will provide technical details about the system. You can use logging as you wish,
we just expect that system operating just fine will not produce a huge volume of messages
at levels INFO or above. This is not a hard requirement, but we will punish *obvious* violations.

## TCP stream state

It should be clear what number of bytes must be processed for valid messages by now.
Here we will specify what must happen in case of errors:

* your solution must slide over bytes in the stream until it detects a valid
magic number—this marks start of the command
* if message type is invalid, discard magic number and the following 4 bytes,
the total of 8 bytes
* every other error consumes the same number of bytes as in case of successful
processing of message of the same type

## Client operations execution

Client operations submitted on TCP can be executed in arbitrary order, and responses
are to be written by system to TCP when a command is completed by the register.

Note that one register (as specified by algorithm in this document) can execute only one operation
at a time (for a given sector). This means operations must be queued. We suggest using TCP
buffer itself as a queue.

## Varia

* We expect that within 300 milliseconds of calling `run_register_process` TCP socket will
be bound. This is for integration tests. We suggest you begin this method with binding
to appropriate socket.
* Note that your internal TCP client cannot lose any messages, even when a target process crashes
and TCP connection gets broken.

## Rust technical details

You may notice `async_trait::async_trait` annotations on top of traits defined
in `solution/src/lib.rs`. Those are necessary to be placed also on top of
corresponding *impl* blocks for those traits inside of your solution—it is a current
technical constraint of Rust, which hopefully will be resolved in the future.

# Testing

Your software must implement the interface, and reliably provide distributed storage despite
crashes. You are given a small subset of official tests. The intention is to make sure that
public interface of your solution is correct, and to reduce friction.

Your solution will be tested with rust version 1.48.0 (7eac88abb 2020-11-16), with stable
features only. Linux kernel version for the driver will be 4.15.0-124-generic.

We suggest that you try to run the complete setup of your system and the driver.

## Grading

Tests are divided in groups. For each group of tests there is some number of points to get
for them. Number of points for each group your solution will receive is proportional to
the total number of tests it will pass. A few simplest test cases disclosed to you
will also be used for final grading.

It will not be able to pass the assignment without actually implementing a distributed solution
(e.g. keeping all data in RAM only, executing commands on a single node only and so forth).

There is an additional task for bonus 3 points mentioned in the `linux_driver.md` file. So it
is possible to get 33 points for this assignment in total.

# Submitting solution

Your solution must be submitted as a single `.zip` file with its name being your
login at students (e.g., ab123456.zip). After unpacking the archive, a directory path
named `ab123456/solution/` must be created. In the `solution` subdirectory there must be a Rust
library crate implementing the required interface. Project `public-tests` shall
be built and tested cleanly when placed next to the `solution` directory.

# Food for thought

This section is outside the realm of marks for the assignment, and there will be no points
for thinking about the issue presented here. You can skip it if you do not have time to grapple
with a design of distributed algorithm.

During the labs we always provide you with a distributed algorithm. This is because creating
*correct* distributed algorithms for fail-recovery model is tough. This assignment is
no different.

However, if you have a lot of spare time, you can think about whether it would be possible to
add atomic (meaning it either has happened or did not from perspective of the client of the system)
compare-and-swap (cas) operation to such distributed register. Would it be possible
to provide meaningful semantics in presence of ordinary writes? Can it be provided with
a single process that can perform cas operations, while every other process can perform writes?
Or is it possible to allow any process to execute both writes and cas and provide meaningful
semantics for reads? (For instance events for algorithm described in the assignment can
be linearized, and it is a useful property).

If you have spare time, you can think about those issues. Remember, there will be no points
for any results of this thinking.

---

Authors: F. Plata, K. Iwanicki, M. Banaszek.
