use super::Rule;
use crate::outbound;

pub struct Direct {}

impl Rule for Direct {
    fn run(&self) -> Option<Box<dyn outbound::Outbound>> {
        unimplemented!()
    }
}
