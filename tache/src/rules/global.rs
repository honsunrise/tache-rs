use crate::outbound;
use crate::rules::ConnectionMeta;

use super::Rule;

pub struct Global {}

impl Rule for Global {
    fn run(&self, cm: &ConnectionMeta) -> Option<&str> {
        Some("DIRECT")
    }
}
