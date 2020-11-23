use std::rc::Rc;

pub struct List<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

impl<T> List<T> {
    /// Creates a new empty list.
    pub fn new() -> Self {
        unimplemented!()
    }

    /// Creates a new list with the new value as head and the old list as tail.
    pub fn cons(&self, _val: T) -> Self {
        unimplemented!()
    }

    /// Length of the list, 0 for empty list.
    pub fn length(&self) -> usize {
        unimplemented!()
    }

    /// Head element if the list is not empty, None if the list is empty.
    pub fn head(&self) -> Option<Rc<T>> {
        unimplemented!()
    }

    /// List without the head element, thus one element shorter. Empty list for empty list.
    pub fn tail(&self) -> Self {
        unimplemented!()
    }
}
