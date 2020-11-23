use std::fmt::Display;

trait TestTrait {
    fn method(&self);
}

// No type is special, trait implementation looks always the same.
impl TestTrait for i32 {
    fn method(&self) {
        println!("Method in trait called: {}", *self);
    }
}

// Trait parametrized by type
trait Print<M>
where
    // Rust will accept only types which are Display
    M: Display,
{
    fn print(&mut self, msg: M);
}

impl<M> Print<M> for String
where
    M: Display,
{
    fn print(&mut self, msg: M) {
        println!("String: {}, message: {}", self, msg);
    }
}

trait AddOne {
    fn add_one(&mut self);
}

trait AddTwo {
    fn add_two(&mut self);
}

pub trait Summary {
    // default implementation
    fn summarize(&self) -> String {
        String::from("(Summary...)")
    }
}

impl Summary for u32 {}

// Trait constraint example.
// Generic implementation of AddTwo, for any type matching constraints.
impl<T> AddTwo for T
where
    T: AddOne + Summary,
{
    fn add_two(&mut self) {
        self.add_one();
        self.add_one();
    }
}

impl AddOne for u32 {
    fn add_one(&mut self) {
        *self += 1;
    }
}

fn trait_constraints() {
    let mut num: u32 = 0;
    num.add_two();
    println!(
        "Method derived from trait constraints. Add two to 0 is: {}",
        num
    );
}

// The definition of Add is copied from the standard library.
trait Add<RHS = Self> {
    type Output;

    fn add(self, rhs: RHS) -> Self::Output;
}

fn main() {
    // Calling method on a 7 might look strange, but if you think about it, there
    // is nothing preventing the compiler from treating it as other struct type.
    7.method();
    "test".to_string().print(7);
    trait_constraints();
    println!("Summary for 7: {}", 7.summarize());
}
