use std::sync::Arc;
use std::collections::HashMap;
use std::error::Error;

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
    fn run(&self) -> Option<&str>;
}

type MODE = Vec<Box<dyn Rule + Send + Sync>>;

pub async fn lookup(modes: Arc<HashMap<String, MODE>>, cm: &ConnectionMeta)
    -> Result<(), Box<dyn Error>> {
    Ok(())
}
