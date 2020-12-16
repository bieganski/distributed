use futures::future::{join_all, ready};
use futures::{FutureExt, TryFutureExt};

fn futures_combining() {
    let future_res = ready(7).map(|x| x * 2);
    println!("{}", futures::executor::block_on(future_res));
    let future_maybe_res: futures::future::Ready<std::result::Result<i32, ()>> =
        futures::future::ok(7);
    println!(
        "{}",
        futures::executor::block_on(future_maybe_res.and_then(|x| futures::future::ok(x * x)))
            .unwrap()
    );
}

fn futures_joining() {
    let futures = vec![ready(1), ready(2), ready(3)];

    assert_eq!(futures::executor::block_on(join_all(futures)), [1, 2, 3]);
}

fn main() {
    futures_combining();
    futures_joining();
}
