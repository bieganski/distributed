/// Macro on top of trait definition.
#[async_trait::async_trait]
trait FutureInt {
    async fn get(&self) -> i32;
}

struct FutureInt5 {}

/// Macro on top of every trait implementation.
#[async_trait::async_trait]
impl FutureInt for FutureInt5 {
    async fn get(&self) -> i32 {
        5
    }
}

fn main() {
    println!(
        "Async trait: {}",
        futures::executor::block_on(FutureInt5 {}.get())
    );
}
