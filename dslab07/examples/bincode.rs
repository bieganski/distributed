/// The example covers the most important part of the API of serde
/// and bincode, and you won't need more.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Entity {
    x: f32,
    y: f32,
}

fn bincode_sample() {
    let e = Entity { x: 1.0, y: -11.0 };
    let encoded: Vec<u8> = bincode::serialize(&e).unwrap();

    let decoded: Entity = bincode::deserialize(&encoded[..]).unwrap();

    assert_eq!(e, decoded);
}

fn main() {
    bincode_sample();
}
