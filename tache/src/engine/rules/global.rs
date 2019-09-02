use super::Rule;
use crate::proxy;

pub struct Global {}

impl Rule for Global {
    fn run(&self) -> Option<Box<dyn proxy::Proxy>> {
        unimplemented!()
    }
}
