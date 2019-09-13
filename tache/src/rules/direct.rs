use super::Rule;
use crate::outbound;

pub struct Direct {}

impl Rule for Direct {
    fn run(&self) -> Option<&str> {
        unimplemented!()
    }
}
