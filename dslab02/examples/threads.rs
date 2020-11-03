use std::thread::spawn;

fn simple_thread() {
    let thread = spawn(|| {
        println!("Inside a thread");
    });

    // Waiting for a thread to finish:
    thread.join().unwrap()
}

fn panicking_thread() {
    let thread = spawn(|| {
        // This will print a long backtrace.
        panic!("Panic attack!")
    });

    // But from the outside, the only result is Err after join.
    assert!(thread.join().is_err())
}

fn moving_values_into_thread() {
    let mut v = vec![1, 2, 3, 4];
    // Not every value can be moved into another thread - they must be Send.
    // Most types are Send, but some like raw pointers and Rc are not.
    let thread = spawn(move || {
        v.push(5);
        println!("{:?}", v);
    });
    thread.join().unwrap();
}

fn main() {
    simple_thread();
    panicking_thread();
    moving_values_into_thread();
}
