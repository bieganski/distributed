use uuid::Uuid;

fn basic_uuid() {
    let id = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
    println!("{}", id);
}

fn uuid_v4() {
    // we suggest using v4 - random ids
    let id_random = Uuid::new_v4();
    println!("{}", id_random);
}

fn main() {
    basic_uuid();
    uuid_v4();
}
