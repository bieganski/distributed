#[macro_use]
pub mod utils {

#[macro_export]
macro_rules! safe_unwrap {
    ($x:expr) => {
        match $x {
            Ok(_) => {},
            Err(_) => log::error!("Internal Error in {} [{}:{}:{}]", module_path!(), file!(), line!(), column!()),
        }
    
    }
}
}