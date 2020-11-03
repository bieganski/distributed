type Task = Box<dyn FnOnce() + Send>;

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::spawn;
use std::thread::JoinHandle;

pub struct Threadpool {
    threads: Vec<JoinHandle<i32>>,
    shared: Arc<(Mutex<Vec<Task>>, Condvar)>,
}

impl Threadpool {
    pub fn new(_workers_count: usize) -> Self {
        let shared = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
        let mut ts = Vec::new();
        

        for _ in 0.._workers_count {
            //let x = shared.clone();
            //let (lock, cond) = x;
            
            // let mut guard = lock.lock().unwrap();
            let not_empty = |v: & Vec<Task>| !v.is_empty();
            // let x = guard.deref();
            //while !predicate(guard.deref()) {

            //}
            //let mut x= lock.lock().unwrap();
            //let &mut guard = &mut *x;
            //while !predicate(guard) {
                // If the predicate does not hold, call `wait`. It atomically
                // releases the mutex and waits for a notify. The while loop is
                // required because of possible spurious wakeups.
           //     let y = cond.wait(x).unwrap();
           // }

           let x = shared.clone(); 
            ts.push(spawn(move || {
                loop {
                let (lock, cond) = &*x; 
                let mut guard = lock.lock().unwrap();
                while !not_empty(guard.deref()) {
                    guard = cond.wait(guard).unwrap();   
                }
                // vec not empty
                let task = (*guard).pop();
                match task {
                    Some(task) => (*task)(),
                    None       => panic!("internal logic error"),
                }
                }
                // return 0;
            }));
        }

        Threadpool {
            threads: ts,
            shared: shared,
        }
        // unimplemented!()
    }

    pub fn submit(&self, _task: Task) {
        // unimplemented!()
        let (lock, cond) = &*self.shared;
        let mut guard = lock.lock().unwrap();
        guard.push(_task);
        cond.notify_all();
    }
}


/*
 *
 * impl Drop for Foo {
    pub fn drop_threads(&mut self) {
        for th in self.handles.drain(..) {
            th.join();
        }
    }
}

*/

impl Drop for Threadpool {
    fn drop(&mut self) {
        while let Some(t) = self.threads.pop() {
            //println!(t);
            t.join().unwrap();
        }
        //for t in &self.threads {
        //    (*t).join().unwrap();
        //}
        // unimplemented!()
    }
}
