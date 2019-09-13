use super::Rule;
use crate::outbound;

pub struct Global {}

impl Rule for Global {
    fn run(&self) -> Option<Box<dyn outbound::Outbound>> {
        unimplemented!()
    }
}
