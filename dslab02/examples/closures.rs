fn plain_closure() {
    // Closure is a function which can be created with a special
    // syntax. The closure below is equivalent to defining
    // an inc function.
    let inc: fn(u32) -> u32 = |x: u32| x + 1;
    println!("plain_closure: {}", inc(inc(0)))
}

fn closure_environment() {
    let allowed_numbers = vec![2, 7];
    let vec = vec![1, 2, 3, 4, 5, 6];
    // This closure is more interesting. It captures its environment.
    // The immutable reference to `allowed_numbers` gets passed into the closure.
    // This is the default capturing of the environment. Note that the closure
    // cannot outlive any of the borrowed references, as any other value in Rust.
    // Every value not in the closure's arguments forms the environment.
    let filtered: Vec<i32> = vec
        .into_iter()
        .filter(|x| allowed_numbers.contains(x))
        .collect();
    println!("closure_environment: {:?}", filtered);
}

// Closure is also a trait. This way, closures can be stored
// in structs and returned from functions.
fn closure_trait() -> Box<dyn Fn(u32) -> bool> {
    let even: Box<dyn Fn(u32) -> bool> = Box::new(|x: u32| x % 2 == 0);
    println!("closure_trait: {:?}", even(8));
    even
}

// There are three different types of closures.
// The FnOnce marks a closure that can be run only once,
// FnMut can modify its environment, and Fn is the broadest
// type for closure, without constraints.
struct ClosureHolder {
    mutable_closure: Box<dyn FnMut(u32) -> ()>,
}

// Fn is also FnMut, and FnMut is also FnOnce.

fn closure_moving_values() {
    let mut vec: Vec<u32> = vec![];
    // Every variable which is not Copy has ownership transferred to the closure.
    // So `move` passes the values into closure, not references.
    // There is no middle ground in Rust, either the whole environment
    // is by reference or by move.
    let mut closure_move = ClosureHolder {
        mutable_closure: Box::new(move |x: u32| {
            vec.push(x);
            println!("closure_moving_values: {:?}", vec);
        }),
    };
    // vec cannot be used anymore.

    (closure_move.mutable_closure)(1);
    (closure_move.mutable_closure)(2);
}

fn closure_fn_once() {
    // It might sound weird that FnOnce is necessary, but consider:
    let v = vec![1, 2];
    // Calling drop_vec twice would access a dropped vector...
    let drop_vec = move || {
        println!("closure_fn_once: {:?}", v);
        drop(v);
    };
    drop_vec();
    // Trying to call again would be a compile error.
    // drop_vec();
}

fn main() {
    plain_closure();
    closure_environment();
    let _ = closure_trait();
    closure_moving_values();
    closure_fn_once();
}
