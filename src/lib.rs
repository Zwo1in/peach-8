#![no_std]

pub mod logger;
pub use logger::{
    Logger,
    Severity,
};

pub mod keeb;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
