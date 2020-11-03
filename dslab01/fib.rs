fn fib(n : u32) -> u32 {

    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main() {
    assert!(fib(0) == 0);
    assert!(fib(1) == 1);
    assert!(fib(2) == 1);
    assert!(fib(3) == 2);
}
