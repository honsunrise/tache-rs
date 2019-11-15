use crate::outbound;
use crate::rules::ConnectionMeta;

use super::Rule;

pub struct Direct {}

impl Rule for Direct {
    fn run(&self, cm: &ConnectionMeta) -> Option<&str> {
        Some("DIRECT")
    }
}
