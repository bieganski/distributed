/// Alternating writing to and reading from buffer_device in two threads.
/// We want to show that operations on the buffer are indeed atomic, and how
/// to interact with files/devices directly.
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::iter::repeat;
use std::path::Path;

static ITERATIONS: u32 = 100000;
const BUF_SIZE: usize = 32;

/// Writing a certain string to a file - strings(index) - and performing a read.
/// Multiple threads execute the same code for different indices.
/// If writes are atomic, we will never read a value not from strings list.
fn write_read_loop(mut handle: File, strings: &[String], index: usize) {
    let handle_ref = &mut handle;

    for _ in 0..ITERATIONS {
        handle_ref.seek(SeekFrom::Start(0)).unwrap();
        handle_ref
            .write_all(strings.get(index).unwrap().as_bytes())
            .unwrap();

        handle_ref.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0 as u8; BUF_SIZE];
        handle_ref.read_exact(&mut buf).unwrap();
        // In this test, only two values could be read
        let _ = strings.iter().find(|&s| s.as_bytes() == buf).unwrap();
    }
}

fn main() {
    let device_path = std::env::args()
        .nth(1)
        .expect("Provide path to buffer device as first argument");
    let handle_first = open_device(device_path.as_ref());
    let handle_second = open_device(device_path.as_ref());

    // a...a - a value to write to the buffer
    let first = repeat('a').take(BUF_SIZE).collect::<String>();
    // b...b - a value to write to the buffer
    let second = repeat('b').take(BUF_SIZE).collect::<String>();

    let strings = vec![first, second];
    let strings_c = strings.clone();

    // Two threads write to the buffer and expect to read always one of the two values.
    let t1 = std::thread::spawn(move || {
        write_read_loop(handle_first, &strings, 0);
    });
    let t2 = std::thread::spawn(move || {
        write_read_loop(handle_second, &strings_c, 1);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("Synchronization OK!");
}

fn open_device(path: &Path) -> File {
    OpenOptions::new()
        .write(true)
        .read(true)
        .open(path)
        .expect(std::format!("Cannot open device: {:?}", path).as_str())
}
