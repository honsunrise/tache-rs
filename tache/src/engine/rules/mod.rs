pub mod direct;
pub mod global;

use crate::outbound;

pub trait Rule {
    fn run(&self) -> Option<Box<dyn outbound::Outbound>>;
}
