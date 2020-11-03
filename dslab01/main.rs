mod solution;

use std::env;
use std::process;

fn parse_args() -> u8 {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => 10,
        2 => match args.get(1).unwrap().parse::<u8>() {
            Ok(n) => n,
            Err(_) => {
                println!("Not an u8 as index to Fibonacci sequence argument!");
                process::exit(1);
            }
        },
        _ => {
            println!("At most one argument - index of Fibonacci number - can be passed!");
            process::exit(1);
        }
    }
}

fn main() {
    println!("{}", solution::fib(parse_args()));
}
