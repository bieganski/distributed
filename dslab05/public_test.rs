#[cfg(test)]
mod tests {
    use crate::solution::List;
    use ntest::timeout;

    #[test]
    #[timeout(100)]
    fn test_list_length() {
        let mut list = List::new();

        for i in 0..10 {
            list = list.cons(i);
            assert_eq!(list.length(), i + 1);
        }
    }
}
