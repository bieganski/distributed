# Small Assignment
Implement a thread pool as described in the previous section. Furthermore, the thread pool must be recognized by Rust as Sync without using unsafe markers.

Your threadpool must be able to process tasks concurrently. We suggest using a mutex with a conditional variable, as shown in the examples.

Also, provide the Drop trait for your pool, it must make sure all worker threads have stopped. Threads in Rust must be stopped in a cooperative manner.

### Type of worker's task
Closures can have a few possible types. In the template, we use

```
type Task = Box<dyn FnOnce<()> + Send>;
```
It specifies a closure which should be called only once (compiler restricts what you can do to achieve this), and that is wrapped in a Box so that the size of Task is known at compile time, the actual closure is on the heap, and it has an additional Send marker. Rust compiler can determine whether a closure is indeed safe to Send at its creation point.


