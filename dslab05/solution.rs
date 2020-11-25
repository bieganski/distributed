use std::rc::Rc;

pub struct List<T> {
    first : Option<Rc<ListValue<T>>>,
}

pub struct ListValue<T> {
    data : T,
    next : Option<Rc<ListValue<T>>>,
}

impl<T> Clone for List<T> {
    
    fn clone(&self) -> Self {
        // let a = match &self.first {
        //     None => None,
        //     Some(rc) => Some(rc.clone()),
        // };
        let res : List<T> = List{first : self.copy_value()};

        return res;
    }
}

impl<T> List<T> {
    /// Creates a new empty list.
    pub fn new() -> Self {
        List{first: None}
    }

    fn copy_value(&self) -> Option<Rc<ListValue<T>>> {
        match &self.first {
            None => None,
            Some(rc) => Some(rc.clone()),
        }
    }

    /// Creates a new list with the new value as head and the old list as tail.
    pub fn cons(&self, val: T) -> Self {
        // let old_copy = self.clone();
        let new_val = ListValue{data: val, 
                                next: self.copy_value()};
        List{first: Some(Rc::new(new_val))}
    }

    /// Length of the list, 0 for empty list.
    pub fn length(&self) -> usize {
        // unimplemented!()
        let mut tmp : Option<&Rc<ListValue<T>>> = self.first.as_ref();
        let mut res : usize = 0;
        while let Some(rc) = tmp {
            res += 1;
            tmp = (*rc).next.as_ref();
        }
        res
    }

    /// Head element if the list is not empty, None if the list is empty.
    pub fn head(&self) -> Option<Rc<T>> {
        unimplemented!()
        // match &self.first {
        //     None => None,
        //     Some(fst_rc) => Some(Rc::new((*fst_rc).data)),
        // }
    }

    /// List without the head element, thus one element shorter. Empty list for empty list.
    pub fn tail(&self) -> Self {
        // unimplemented!()
        match &self.first {
            None => List{first: None},
            Some(rc) => List{first: (*rc).next.clone()}
        }
    }
}
