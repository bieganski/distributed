// Invalid Rust code for selecting longer string.
//
// Both references have distinct implicit lifetime parameters,
// and the compiler is unable to deduce lifetime of the result.
//
// fn longest(x: &str, y: &str) -> &str {
//    if x.len() > y.len() {
//        x
//    } else {
//        y
//    }
// }


// This issue can be fixed by stating that both references (and the result)
// have the same lifetime:
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

fn main() {
    let string1 = String::from("long string is long");

    {
        let string2 = String::from("xyz");
        let result = longest(string1.as_str(), string2.as_str());
        println!("The longest string is {}", result);

        // Lifetime of `result` is deduced to be the same as of `string2`.
        // Should we try to return `result` here, the compiler will complain about
        // `string2` not living long enough.
        //
        // result
    }
}
