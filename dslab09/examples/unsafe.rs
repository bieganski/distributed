use std::rc::Rc;

/// Function contains unsafe blocks, but does not need
/// to be marked with unsafe itself. As always with unsafe,
/// you must ensure that there will be no programming error.
fn dereferencing_raw_pointer() {
    let mut num = 5;

    let r1 = &num as *const i32;
    let r2 = &mut num as *mut i32;

    println!("num is: {}", num);
    unsafe {
        // This works because num is also in scope.
        println!("r1 is: {}", *r1);
    }
    unsafe {
        // num is still in scope.
        *r2 = 6;
    }
    println!("num is: {}", num);
    unsafe {
        // Again, num is still alive so this is fine.
        println!("r1 is: {}", *r1);
    }
}

static mut CTR: u32 = 0;

// This function is actually OK, but only when one thread of execution
// calls it – but then, there is no point in a global static...
// It looks innocent, but += 1 is not an atomic operation and calling this
// from multiple threads would not result in a deterministic value of CTR.
unsafe fn global_static() {
    CTR += 1;
    println!("CTR is: {}", CTR);
    CTR += 1;
    println!("CTR is: {}", CTR);
}

#[derive(Clone)]
struct NotSync {
    data: Rc<u8>,
}

/// This is not the case in this examples, but sometimes
/// you know that part of a type which is not Sync is used only
/// in one thread – because you created the code – and the type is
/// Sync, even though the compiler cannot know that. Here Rc is unsafe,
/// because it uses non-thread safe reference counting.
unsafe impl Sync for NotSync {}
/// Sending Rc is not safe, again because reference counting is not atomic.
unsafe impl Send for NotSync {}

// This is actually OK. First, the counter is bumped only in the main thread.
// Secend, the internal thread ends before the function ends, so the reference
// counter is decreased in the main thread (in Drop) strictly after the internal
// tread ends decreasing it.
// But isn't it just easier to use Arc?
fn unsafe_sync() {
    let not_sync = NotSync { data: Rc::new(1) };
    let nsc = not_sync.clone();

    let thread = std::thread::spawn(move || {
        println!("Data: {:?}", nsc.data);
    });

    println!("Data: {:?}", not_sync.data);
    thread.join().unwrap();
}

/// You might have seen unions in C or C++. They are a mechanism to
/// interpret a single value (a chunk of bytes) differently in different
/// context. Size of the union is size of its maximum element in bytes,
/// and a write to one field may change the value of other field. They allow
/// to pack bytes tightly.
/// repr(C) specifies that Rust must generate memory layout of the struct
/// the same as the C compiler would – it is handy when using C code.
#[repr(C)]
union MyUnion {
    f1: u32,
    f2: f32,
}

fn union_example() {
    let u = MyUnion { f1: 1 };
    // This totally OK, just reading union.
    unsafe { println!("Union field 2: {}", u.f2) };
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Complex {
    re: f32,
    im: f32,
}

/// The attribute `link` specifies that we want to link with libm, which should be
/// installed on the system. It is also possible to define in `build.rs` additional
/// steps which will compile some C code to be used in Rust project.
#[link(name = "m")]
extern "C" {
    // Cosinus of a complex number.
    fn ccosf(z: Complex) -> Complex;
}

// Since calling foreign functions is considered unsafe,
// it's common to write safe wrappers around them.
// Correct bytes are inserted to method, so this call will
// work just fine.
fn cos(z: Complex) -> Complex {
    unsafe { ccosf(z) }
}

impl std::fmt::Debug for Complex {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.im < 0. {
            write!(f, "{}-{}i", self.re, -self.im)
        } else {
            write!(f, "{}+{}i", self.re, self.im)
        }
    }
}

fn foreign_function_interface() {
    let z = Complex { re: -1., im: 0.2 };
    println!("cos({:?}) = {:?}", z, cos(z));
}

fn main() {
    dereferencing_raw_pointer();
    unsafe { global_static() };
    unsafe_sync();
    union_example();
    foreign_function_interface();
}
