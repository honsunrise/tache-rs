use crate::rules::ConnectionMeta;

use super::Rule;

pub struct Direct {}

impl Rule for Direct {
    fn run(&self, _cm: &ConnectionMeta) -> Option<&str> {
        Some("DIRECT")
    }
}
