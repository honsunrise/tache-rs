use crate::rules::ConnectionMeta;

use super::Rule;

pub struct Global {}

impl Rule for Global {
    fn run(&self, _cm: &ConnectionMeta) -> Option<&str> {
        Some("DIRECT")
    }
}
