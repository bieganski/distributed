// example import from the standard library
use std::ops::Add;

/// A simple function is presented here.
///
/// Note that Rust's doc comments start with `///` and support **Markdown**!
/// They can be then easily converted (by calling `cargo doc` which internally
/// launches `rustdoc`) to a website, just like the
/// _https://doc.rust-lang.org/std/index.html_. Moreover, the doc comments may
/// include examples which can be then automatically tested!
fn hello_world() {
    // println! is a macro for printing.
    // No semicolon at the end, we return the result of calling the macro.
    println!("Hello world!")
}

fn may_panic() {
    // Result<T, E> is a return type for all operations that may fail.
    // It represents either correct result or failure.
    let parsed_or_not = "7".to_string().parse::<u8>();
    // unwrap calls panic! when result is failure.
    // A panic is a critical condition in software. Its behavior
    // can be implemented, the default on a fully fledged OS is stack unwinding,
    // on a microcontroller this might be just infinite loop or a hardware reset
    println!("Parsed: {}", parsed_or_not.unwrap());
}

// one warning about unused function less...
#[allow(dead_code)]
fn halting_problem(_machine_description: String) {
    // another useful macro, it calls panic, marks some TODO
    unimplemented!()
}

fn mutability() {
    let _x = 6;
    // line below would be a compilation error
    // x = 5;
    // redeclearing variable is fine, here a mutable one
    let mut x = 6;
    x += 1;
    println!("x: {}", x)
}

fn takes_ownership(mut s: String) {
    s.push('t');
    println!("Owns string: {}", s)
}

fn ownership_flow() {
    let s = "tes".to_string();
    takes_ownership(s)
    // println!(s) here would be a compilation error
}

fn references(s1: &mut String, s2: &str) {
    s1.push('c');
    println!("s1: {}; s2: {}", s1, s2);
}

fn passing_ref() {
    let mut s1 = "ab".to_string();
    let s2 = "xyz";

    references(&mut s1, s2);
}

// This is a struct definition, similar to C
#[derive(Debug, Copy, Clone)]
struct ChattyInteger {
    num: i32,
}

// Implementing trait for a structure. This trait serves
// two purposes: it can be used as any other trait, and also
// will allow for `+` to be used on two ChattyIntegers.
impl Add for ChattyInteger {
    // Self refers to `ChattyInteger`.
    // Traits can not only have associated methods, but also types.
    type Output = Self;

    fn add(self, other: Self) -> Self {
        println!("Adding...");
        // Creation of a struct
        Self {
            num: self.num + other.num,
        }
    }
}

fn chatty_integer_addition() {
    let i1 = ChattyInteger { num: 7 };
    let i2 = ChattyInteger { num: 8 };
    let sum = i1 + i2;
    // Formatting print using Debug macro
    println!("Sum {:#?}", sum)
}

// Rust has enums
enum ProcessingUnit {
    // Option with unnamed fields
    CPU(u32),
    // Option with named fields, looks similar to struct
    // Lifetime parameter of references
    GPU { frequency: u128, name: &'static str },
    // Option without fields
    TPU,
}

// function with return type, using enum
fn calculate_cost(pr: ProcessingUnit) -> u32 {
    // simple match for enums. See how each branch has a value of the
    // last expression, and this results in the whole match being an expression.
    let cost_unit = match pr {
        ProcessingUnit::CPU(v) => v,
        ProcessingUnit::GPU {
            frequency: _freq,
            name,
        } => name.len() as u32,
        ProcessingUnit::TPU => 7,
    };

    // integers can be pattern matched too
    match cost_unit {
        0x00..=0xFF => 11,
        _ => 8,
    }
}

// generic list as enum. Every distinct type will get its own
// compiled version of this enum, like in C++
#[derive(Debug)]
enum List<T> {
    Head(T, Box<List<T>>),
    Nil,
}

// Standalone functions can be generic too
fn add_element_to_list<T>(list: List<T>, elem: T) -> List<T> {
    List::Head(elem, Box::new(list))
}

// Implementing a trait with type parameters, and trait bound.
// T must have a Clone trait.
impl<T> Clone for List<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            List::Head(val, list) => List::Head(val.clone(), list.clone()),
            List::Nil => List::Nil,
        }
    }
}

fn main() {
    hello_world();
    may_panic();
    mutability();
    ownership_flow();
    passing_ref();
    chatty_integer_addition();
    println!(
        "Calculated cost: {}",
        calculate_cost(ProcessingUnit::TPU)
            + calculate_cost(ProcessingUnit::GPU {
                frequency: 10,
                name: "name"
            })
            + calculate_cost(ProcessingUnit::CPU(11))
    );
    println!(
        "Adding integer to list: {:#?}",
        add_element_to_list(List::Nil, 7)
    );
    println!(
        "Adding string to list: {:#?}",
        add_element_to_list(List::Nil, "test".to_string())
    );
}
