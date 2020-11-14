mod public_test;
mod solution;

use std::env;
use std::process;

fn parse_args() -> usize {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => 10,
        2 => match args.get(1).unwrap().parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Not an unsigned number as program argument!");
                process::exit(1);
            }
        },
        _ => {
            println!("Only one argument - index of fibonacci number - can be passed!");
            process::exit(1);
        }
    }
}

fn main() {
    solution::fib(parse_args());
}
