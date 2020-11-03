use std::fmt;

fn copy_example() {
    let v: u32 = 7;
    fn use_int(mut v: u32) {
        v += 1;
        println!("{}", v)
    }
    use_int(v);
    // Normally v would be invalid, but u32 has trait Copy
    // which means that Rust can safely copy it byte by byte,
    // so from Rust perspective another value is created.
    use_int(v);
}

#[derive(Clone)]
struct SimpleInt {
    num: u32,
}

// Compiler would protest if SimpleInt did not contain
// only types which are Copy. Copy is a type of Clone
// (Copy is byte-by-byte, Clone can be done in a special manner).
// Because of this, if T is Copy, it must be also Clone.
impl Copy for SimpleInt {}

struct Cloneable {
    data: Vec<u8>,
}

// Clone creates an equivalent value - in our case, it duplicates
// the vector. It is never used automatically, you always
// must call it explicitly, as it might be expensive.
impl Clone for Cloneable {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

fn clone_example() {
    let vec = vec![1, 2, 3, 4, 5];
    fn own_vector(mut v: Vec<i32>) {
        *v.get_mut(0).unwrap() = 7;
    }
    own_vector(vec.clone());
    // No change visible here.
    // This also gets us to the Debug trait.
    println!("{:#?}", vec);
}

// An implementation of Debug describes how to build a debug-string
// from your type. Rarely there is a reason not to just derive it.
impl fmt::Debug for SimpleInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimpleInt Test")
            .field("number", &self.num)
            .finish()
    }
}

fn debug_example() {
    let simple_int = SimpleInt { num: 7 };

    // Just look at the output to get the difference.
    println!("{:?}", simple_int); // Debug
    println!("{:#?}", simple_int); // pretty-printed Debug
}

// Now the magic that automatically frees memory for Box and Rc.
// Drop trait has only one method - drop - and gets it called automatically
// when the value goes out of scope. It can free memory or close a file.
// Drop for fields of a struct gets called automatically, so you only need
// to worry about 'resources' introduced by your struct.

struct Droppable {
    name: &'static str,
}

impl Drop for Droppable {
    fn drop(&mut self) {
        println!("> Dropping {}", self.name);
    }
}

fn drop_example() {
    {
        let _d = Droppable { name: "test value" };
        // Rust does not allow explicit calls to drop:
        // _d.drop();
    } // _d gets dropped at the end of scope.
      // Open directory.
    let dir_path = std::env::current_dir().unwrap();
    // Get files in the dir.
    let dir_filepaths = std::fs::read_dir(dir_path).unwrap();
    for filepath in dir_filepaths {
        // We open a file:
        let _file = std::fs::File::open(filepath.unwrap().path()).unwrap();
        // There is no close in Rust.
        // Drop takes care of closing a file descriptor for us.
    }
}

fn main() {
    copy_example();
    clone_example();
    debug_example();
    drop_example();
}
