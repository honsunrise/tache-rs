use super::Rule;
use crate::outbound;

pub struct Global {}

impl Rule for Global {
    fn run(&self) -> Option<&str> {
        unimplemented!()
    }
}
