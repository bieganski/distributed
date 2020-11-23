use std::fmt::Formatter;

pub trait NewTrait {}

struct MyInt {
    int: i32,
}


// You can implement a local trait on a type from other crate.
impl NewTrait for u32 {}

// You can implement a trait from other crate on the local type.
impl std::fmt::Debug for MyInt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("MyInt ")?;
        self.int.fmt(f)
    }
}

// But you cannot implement a trait from other crate on a type from other crate.
//
// This is compile time error:
//
//impl std::fmt::Debug  for uuid::Uuid {
//    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//        unimplemented!()
//    }
//}


// The result type means: any type inside Box that implements Debug trait,
fn builder() -> Box<dyn std::fmt::Debug> {
    Box::new(MyInt { int: 7 })
}

trait Polymorphic {
    fn poly<A>(&self, arg: A) -> A;
}

impl Polymorphic for u32 {
    fn poly<A>(&self, arg: A) -> A {
        arg
    }
}

// Rust cannot build trait object with polymorphic methods:
//
// This is compile time error:
//
//fn build_trait_with_polymorphic_function() -> Box<dyn Polymorphic> {
//    Box::new(0)
//}


fn main() {
    println!("{:?}", builder());
}
