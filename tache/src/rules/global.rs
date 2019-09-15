use super::Rule;
use crate::outbound;
use crate::rules::ConnectionMeta;

pub struct Global {}

impl Rule for Global {
    fn run(&self, cm: &ConnectionMeta) -> Option<&str> {
        Some("DIRECT")
    }
}
