pub fn fib(_n: u8) -> u128 {
    if _n == 0 {
        return 0;
    }
    if _n == 1 {
        return 1;
    }
    return fib(_n - 1) + fib(_n - 2);
}

pub fn fib_test() {
    assert!(fib(0) == 0);
    assert!(fib(1) == 1);
    assert!(fib(2) == 1);
    assert!(fib(3) == 2);
}
