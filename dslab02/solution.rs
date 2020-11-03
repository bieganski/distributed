type Task = Box<dyn FnOnce() + Send>;

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::spawn;
use std::thread::JoinHandle;

pub struct Threadpool {
    threads: Vec<JoinHandle<()>>,
    shared: Arc<(Mutex<Vec<Option<Task>>>, Condvar)>,
}

impl Threadpool {
    pub fn new(_workers_count: usize) -> Self {
        let shared = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
        let mut ts = Vec::new();
        
        let not_empty = |v: & Vec<Option<Task>>| !v.is_empty();
        
        for _ in 0.._workers_count {
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
                        Some(task) => match task {
                            Some(task) => task(),
                            None       => return, 
                        },
                        None       => panic!("internal logic error"),
                    }
                }
            }));
        }

        Threadpool {
            threads: ts,
            shared: shared,
        }
    }

    pub fn submit(&self, task: Task) {
        let (lock, cond) = &*self.shared;
        let mut guard = lock.lock().unwrap();
        guard.push(Some(task));
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
        for _ in &self.threads {
            let (lock, cond) = &*self.shared;
            let mut guard = lock.lock().unwrap();
            guard.push(None);
            cond.notify_all();
        }
        while let Some(t) = self.threads.pop() {
            t.join().unwrap();
        }
    }
}
