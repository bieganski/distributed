#[cfg(test)]
mod tests {
    use crossbeam_channel::unbounded;
    use crate::solution::{EngineMessage, FibonacciModule};

    #[test]
    fn new_registers_module() {
        let (tx, rx) = unbounded();
        FibonacciModule::create(0, 7, tx);
        assert_eq!(rx.len(), 1);
        match rx.try_recv().unwrap() {
            EngineMessage::RegisterModule(_) => {}
            _ => panic!("Creating module resulted in different message than RegisterModule!"),
        }
    }
}
