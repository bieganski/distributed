use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::spawn;

// A type T is Sync, when &T is Send.
fn immutable_data_is_sync() {
    // Arc<T> is Send + Sync when T is Sync.
    let vec = Arc::new(vec![1, 2, 3]);
    let vec_clone = vec.clone();

    let t1 = spawn(move || {
        println!("immutable_data_is_sync: {:?}", vec_clone);
    });
    let t2 = spawn(move || {
        println!("immutable_data_is_sync: {:?}", vec);
    });

    t1.join().unwrap();
    t2.join().unwrap();
}

fn immutable_mutex_reference_grants_mutable_access() {
    fn modify_data_behind_mutex(data: &Mutex<Vec<u32>>) {
        let mut guard = data.lock().unwrap();
        let mut_ref: &mut Vec<u32> = guard.deref_mut();
        mut_ref.push(1);
    }
    let safe_data = Mutex::new(vec![0]);
    modify_data_behind_mutex(&safe_data);
    println!(
        "immutable_mutex_reference_grants_mutable_access: {:?}",
        safe_data.lock().unwrap()
    );
}

fn atomic_reference() {
    // Arc<Mutex<T>> is a hammer for shared data in Rust.
    let send_and_sync = Arc::new(Mutex::new(vec![1, 2, 3]));
    let cloned = send_and_sync.clone();

    let thread = spawn(move || {
        let mut guard = cloned.lock().unwrap();
        let data = guard.deref_mut();
        data.push(4);
        println!("atomic_reference: {:?}", data);
    });

    // cloned was moved to the closure, but we can still use send_and_sync here:
    send_and_sync.lock().unwrap().push(5);

    thread.join().unwrap();
}

fn conditional_variable() {
    // Design pattern is to associate conditional variable with a mutex.
    let shared = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
    let cloned = shared.clone();
    let predicate = |vec: &Vec<i32>| !vec.is_empty();

    // Standard Condvar use, one thread does this:
    let thread = spawn(move || {
        let (lock, cond) = &*cloned;
        // Locking a mutex:
        let mut guard = lock.lock().unwrap();
        while !predicate(guard.deref()) {
            // If the predicate does not hold, call `wait`. It atomically
            // releases the mutex and waits for a notify. The while loop is
            // required because of possible spurious wakeups.
            guard = cond.wait(guard).unwrap();
        }
        // Predicate holds and mutex is locked here.
        // ...
    });

    // While the other thread:
    {
        let (lock, cond) = &*shared;
        let mut guard = lock.lock().unwrap();
        guard.push(0);
        // Wake up every thread waiting on the variable.
        cond.notify_all();
    }

    thread.join().unwrap();
}

fn main() {
    immutable_data_is_sync();
    immutable_mutex_reference_grants_mutable_access();
    atomic_reference();
    conditional_variable();
}
