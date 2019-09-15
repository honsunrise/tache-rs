use std::sync::Arc;
use std::collections::HashMap;
use std::error::Error;
use crate::Config;
use global::Global;
use direct::Direct;

pub mod direct;
pub mod global;

#[derive(Debug, Clone)]
pub struct ConnectionMeta {
    pub udp: bool,
    pub host: String,
    pub src_addr: Option<std::net::SocketAddr>,
    pub dst_addr: Option<std::net::SocketAddr>,
}

impl ConnectionMeta {
    pub fn is_host(&self) -> bool {
        !self.host.is_empty()
    }
}

pub trait Rule {
    fn run(&self, cm: &ConnectionMeta) -> Option<&str>;
}

pub type MODE = Vec<Box<dyn Rule + Send + Sync>>;

pub fn build_modes(config: &Config) -> Result<HashMap<String, Arc<MODE>>, Box<dyn Error>> {
    let mut result: HashMap<String, Arc<MODE>> = HashMap::new();
    // build buildin mode
    result.insert("GLOBAL".to_owned(), Arc::new(vec![Box::new(Global {})]));
    result.insert("DIRECT".to_owned(), Arc::new(vec![Box::new(Direct {})]));
    // build rule mode
    let mut rules = vec![];
    result.insert("RULE".to_owned(), Arc::new(rules));

    Ok(result)
}

pub async fn lookup(mode: Arc<MODE>, cm: &ConnectionMeta)
                    -> Result<String, Box<dyn Error>> {
    for rule in mode.iter() {
        if let Some(outbound) = rule.run(cm) {
            return Ok(outbound.to_owned());
        }
    }
    Err(From::from("cant't find match rule"))
}
