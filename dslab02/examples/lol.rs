use std::rc::Rc;
use std::sync::Arc;

fn oops() {
    let rc = Arc::new(1);
    let rc_clone = rc.clone();
    let thread = std::thread::spawn(move || {
        let v = *rc_clone;
        // ...
    });
}

fn main() {
// oops();

let a : (u32, u32) = (10, 11);
let x = &a;
let (b, c) = &*x;
print!("a : {:#?}", a);
}
