```shell
cargo test --test # it will list possible tests
cargo test --test <testname.rs>
```


Some tests may demand nighly version of rust. You need to install it and run cargo in `+nightly` mode.

```shell
rustup install nightly
cargo +nightly install racer

cargo +nightly test --test <testname.rs>
```


Also, keep in mind, that few tests (like `multiple_nodes.rs`) may need additional dependencies (all are listed in `Cargo.toml` file).
