use siphasher::sip::SipHasher24;
use std::hash::{Hash, Hasher};

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Config {

}

impl Default for Config {
    fn default() -> Self {
        Config {
        }
    }
}

impl Config {
    pub fn merge_file(&mut self, file: ConfigFile) {

    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct ConfigFile {
}
