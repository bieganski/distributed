struct SimplestGeneric<T>(T);

fn simplest_generic_struct() {
    let a = SimplestGeneric('a');
    let b = SimplestGeneric(7);
    let c = SimplestGeneric(7.0);

    println!("a: {}, b: {}, c: {}", a.0, b.0, c.0);
}

fn largest_array_elem<T: std::cmp::Ord>(array: &[T]) -> &T {
    let mut result = &array[0];
    for i in 1..array.len() {
        if &array[i] > result {
            result = &array[i];
        }
    }
    result
}


struct Stack<T> {
    data: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Self {
        Stack { data: Vec::new() }
    }

    fn push(&mut self, val: T) {
        self.data.push(val);
    }

    fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }
}

// Implementing a trait for generic type.
impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn stack_example() {
    let mut stack = Stack::new();
    stack.push(3);
    stack.push(2);
    stack.push(1);

    while let Some(val) = stack.pop() {
        println!("Value on stack: {}", val)
    }
}


//  It is also possible to constraint type parameter here.
enum MyOptional<T>
where
    T: Eq + PartialEq,
    // M: Hash,
    // ...
{
    MySome(T),
    MyNothing,
}

fn generic_enum() {
    let e = MyOptional::MySome(7);
    // `()` is also a type
    let _n: MyOptional<()> = MyOptional::MyNothing;

    match e {
        MyOptional::MySome(v) => println!("In enum: {}", v),
        MyOptional::MyNothing => panic!("Will not happen!"),
    }
}

fn main() {
    simplest_generic_struct();
    println!("Largest in array: {}", largest_array_elem(&[1, 7, 2, 4]));
    stack_example();
    generic_enum();
}
