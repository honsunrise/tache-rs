use super::Rule;
use crate::outbound;
use crate::rules::ConnectionMeta;

pub struct Direct {}

impl Rule for Direct {
    fn run(&self, cm: &ConnectionMeta) -> Option<&str> {
        Some("DIRECT")
    }
}
