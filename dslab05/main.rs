use crate::solution::List;
use std::fmt::Display;

mod public_test;
mod solution;

fn print_list<T: Display>(mut list: List<T>) {
    println!("Printing a list");
    while list.length() > 0 {
        println!("List elem: {}", list.head().unwrap());
        list = list.tail();
    }
    println!();
}

fn main() {
    let list_empty = solution::List::new();
    assert_eq!(list_empty.tail().length(), 0);

    let list_1 = list_empty.cons(1);
    let list_2 = list_1.cons(2);
    let list_3 = list_2.cons(3);

    print_list(list_3.clone());
    print_list(list_1);

    assert_eq!(list_3.length(), 3);
    assert_eq!(list_2.tail().length(), 1);
}
