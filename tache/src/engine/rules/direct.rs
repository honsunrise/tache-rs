use super::Rule;
use crate::proxy;

pub struct Direct {}

impl Rule for Direct {
    fn run(&self) -> Option<Box<dyn proxy::Proxy>> {
        unimplemented!()
    }
}
