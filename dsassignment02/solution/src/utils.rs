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

#[macro_export]
macro_rules! safe_err_return {
    ( $msg_str:expr ) => {
        {
            log::error!("{} in {} [{}:{}:{}]", $msg_str, module_path!(), file!(), line!(), column!());
            Err(std::io::Error::from(std::io::ErrorKind::Other{}))
        }
    };
}
}