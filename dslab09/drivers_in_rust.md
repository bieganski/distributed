# Safe and unsafe code in Rust

Let us start by discussing here a more advanced Rust's feature of *unsafe* code. Unsafe
code must only be used when necessary, because some properties of Rust are not checked
in unsafe code.

So the rule of thumb would be not to use what is described here. It is necessary
when wrapping legacy API coded in other programming language than Rust, and when working
directly with hardware. We will come back to this after describing the topic at hand.

Examples are in `unsafe.rs`.

## Raw pointers

We begin with an introduction of raw pointers. Essentially, on memory level these are the same
as references in Rust. However, there are no guarantees enforced by the borrow checker—it
is possible to have two unrelated mutable pointers to the same data (for example: `*mut i32`).
Because of this, dereferencing them is not safe in Rust, and it cannot be done in *safe*,
standard code. However, merely creating pointers is safe. There are also immutable pointers,
for example `*const i32`. Inside `unsafe` blocks of code you can dereference both
mutable and immutable pointers—only the first one will yield a mutable value.

As in C, pointers can have issues. Pointers can be null, they can point to arbitrary memory, and
their *Drop* does nothing. When using raw pointers and `unsafe`, you are on your own.

## Unsafe

There is a keyword `unsafe` in Rust. It is used to mark functions and blocks of code which
have more loose requirements than the rest of Rust code. Sometimes Rust with its guarantees
can be too restrictive, for instance, it would be impossible to interact with hardware using
language constructs of Rust that were shown during the previous labs. Sometimes you might know that
your code is correct—e.g. that a type is *Sync*—but the compiler cannot infer it. This is where
you might need `unsafe`.

Before we dive in into examples, and what operations `unsafe` allow, we begin with
a remark that `unsafe` blocks of code should be isolated, and it is best when their functionality
is provided via a safe Rust interface. For instance, one can create a *struct* which
represents some hardware on microcontroller, and provide a *safe* interface with standard
Rust methods to call. Only the implementation of such modules and types needs to invoke
`unsafe`. Should you use `unsafe` too much, you will lose many advantages of Rust.

`unsafe` blocks allow you perform following operations:

* dereference a raw pointer
* call other unsafe code—foreign function interface code is also unsafe
(e.g. a function in C called from Rust)
* access and modify global mutable statics
* implement unsafe trait (like mark type *Sync*)
* access fields of unions

All other rules of Rust stay intact—type safety is not violated.

## Potential side effects of unsafe

This is a short list of what can go wrong with unsafe:

* dereferencing unaligned pointer—this might crash on *some* CPUs and not others
* a data race
* creating invalid value, e.g. bool which is not 0 or 1, or invalid char
* double free

This is not to say that your code will certainly have these issues—one can code a perfect
program in assembly. It is just arguably more time-consuming, and for sure less portable than
coding equivalent software in C. You can think of unsafe Rust as of C with more strict
type checker. So when you use unsafe, you lose almost all benefits of coding in Rust.
Because of this, it should be used only when necessary, and not because you want to take
a shortcut to work around some issue which could be resolved with better design. But if
you must use it, then do it and be very careful about the code in unsafe blocks.

# Drivers in Rust

We will be referencing `rust_ram_disk_driver/` example.

## Rust in Linux drivers

Rust can be used without its standard library (in main file of binary of library
you need to include `#![no_std]`), making it possible to run the code in environments
without operating system. Such Rust code can be run directly on microcontrollers
or inside the Linux kernel. We will use [linux-kernel-module-rust](https://github.com/fishinabarrel/linux-kernel-module-rust)
which is a project which provides a few things:

* automatic generation of bindings to Linux kernel API from headers which can be
used in Rust code
* allocator wrapping *kmalloc*—this makes it possible to use collections
from *alloc* crate, like `Vec`
* wrappers in Rust around Linux APIs, making them more Rust-like

Experimental features of Rust are used, which means the *stable* compiler will not be able to
build it. We need *nightly* compiler to build it. This project is build in CI
(continuous integration) which makes it much easier to reproduce builds. Here we provide
instructions based on `linux-kernel-module-rust/.travis.yml` from the time of writing
(commit hash `b355d71dc7c3b4384cee6811ac92ce3527a0c650`):

* clone this repository and preferably do checkout to aforementioned commit hash
* use virtual machine, Ubuntu 18.04.4 (we suggest server edition, as GUI is not needed)
* install `clang-9` and make the `clang` be default (this comes from travis website)
* install rust and switch it to `nightly-2020-07-12`
* install Linux headers and coreutils
* install Rust components: rust-src and rustfmt
* modify `src/bindings_helper.h` by adding `#include <linux/blkdev.h>`—the project
does not have experimental support for block devices but we will need this later on

Then you should be able to build the provided `linux-kernel-module-rust/hello-world` example.
The project is not easy to build, it is picky about kernel, Rust and clang versions. There is
no support in Linux kernel directly for creating drivers, but the idea has been gaining some
traction with a dedicated workshop in July 2020 for this issue. So one day all of
this might be much easier.

## Why to use Rust in the drivers

You probably have noticed patterns how errors in kernel modules are handled.
This is very similar to what the Drop trait does automatically. Rust simplifies also
handling of explicit errors via *Result* type, and the question mark `?` macro to write compact
code which handles a lot of errors—a lot of code in the kernel does error handling
(we do not have a number here to quote) because if anything goes wrong, the whole kernel
can be impacted. In `linux-kernel-module-rust` there is a type *KernelResult*
which represents the possibility of an error.

Synchronization primitives like mutexes and spinlocks should be used to protect
piece of data that is to be accessed concurrently. Rust can enforce that statically via generic
wrapper types, which makes it impossible to forget to acquire a lock.

Some parts of the kernel must not call APIs which may result in blocking—for instance,
you must not block when holding a spinlock, or inside an interrupt handler (think of it as of
a callback run when hardware issues a notification). Languages like Rust (think of Sync, Send),
possibly after some extension of the language itself, could make it possible to enforce
those constraints at compile time.

Kernel uses reference counting for automatic management—this can be done automatically
via a type similar to *Arc*.

All of this makes it reasonable to at least play with Rust in kernel space,
and kernel modules are a great playground. The main problem with creating Rust API is that
some code in Linux is based on C macros which are hard to automatically translate into
Rust bindings. Because of that, we need `macro_wrapper.c` which
exports some additional functions (plain C functions can be easily called from Rust).

You will use `linux-kernel-module-rust project` to build a driver for simple RAM block device
in Rust. Before that, you will read what block devices are, and what APIs exist for them.

## Block devices in Rust

We will be referencing `rust_ram_disk_driver/blkdev.rs` in this and the next sections.
It is an example of how to wrap Linux API into Rust API—with the goal to constraint
`unsafe` blocks to API implementation.

### Registering a block device

The registration is done by `build` method which returns *Registration* struct.
There is no need to remember about deregistering—the `drop` method of *Registration*
takes care of it.

### Registering a block device

Generic block device information is represented by *BlockDevice* struct.

### Block device operations

Block devices are special, and different from other files. Two methods we have to provide
are `open` and `release`, and those do nothing in `blkdev.rs`. There is ***no*** `read`
or `write` methods, transferring of data is done via request queues.

# Further reading

If you wish to learn more about `unsafe` code, the best guide would be
[the Rustonomicon](https://doc.rust-lang.org/nomicon/index.html).

And if you are interested in how Rust could be use for lowest-level code and in
operating systems in general, there is a blog about creating
[an extremely simple operating system in Rust](https://os.phil-opp.com/).
It contains a good example of wrapping hardware into *safe* and efficient
abstractions: a VGA text buffer, and simple wrapping interrupt handlers with `async` tasks.

There is also a toy operating system coded in Rust, [Redox](https://www.redox-os.org/). They
boast about having GUI, but we have not tried to run it.

---

Authors: F. Plata, K. Iwanicki, M. Banaszek.
