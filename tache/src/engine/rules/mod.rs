pub mod direct;
pub mod global;

use crate::proxy;

pub trait Rule {
    fn run(&self) -> Option<Box<dyn proxy::Proxy>>;
}
