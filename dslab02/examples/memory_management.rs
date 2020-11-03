use std::ops::Deref;
use std::rc::Rc;

fn box_example() {
    let array_on_heap: Box<[u32]> = Box::new([1, 2, 3, 4, 5, 42]);

    // Iterate over array_on_heap. The i variable has type &T.
    for i in array_on_heap.iter() {
        println!("Integer from heap: {}", i);
    }

    // Explicit use of deref is not needed, but here we provide it
    // for complete clarity. It explicitly converts Box<T> into &T.
    for i in array_on_heap.deref().iter() {
        println!("Integer from heap: {}", i);
    }

    // You can define functions inside functions!
    fn take_box_ownership(mut boxed: Box<[u32]>) {
        // Here we dereference with '*'.
        *boxed.get_mut(0).unwrap() = 14;
        println!("Owned box: {:#?}", boxed);
    }
    take_box_ownership(array_on_heap);
    // array_on_heap cannot be used anymore
}

fn rc_example() {
    let array_on_heap: Rc<[u32]> = Rc::new([1, 2, 3, 4, 5, 42]);

    for i in array_on_heap.iter() {
        println!("Integer from heap: {}", i);
    }

    for i in array_on_heap.deref().iter() {
        println!("Integer from heap: {}", i);
    }

    fn take_rc_ownership(rc: Rc<[u32]>) {
        // You cannot mutate data inside Rc, because otherwise
        // there could be multiple mutable references to the same data.
        // (If you need such a structure read about Cell/RefCell).
        println!("Owned rc: {:#?}", rc);
        // The counter is decreased automatically by Rust.
    }
    // Calling clone bumps the counter.
    take_rc_ownership(array_on_heap.clone());
    // array_on_heap is still a valid variable to use.
    take_rc_ownership(array_on_heap);
}

fn main() {
    box_example();
    rc_example();
}
