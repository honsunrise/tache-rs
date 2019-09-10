//! Shadowsocks Server Context

use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use lru_cache::LruCache;
use trust_dns_resolver::Resolver;

use crate::{config::Config, engine::dns_resolver::create_resolver};

type DnsQueryCache = LruCache<u16, (SocketAddr, Instant)>;

#[derive(Clone)]
pub struct Context {
    config: Config,
    dns_resolver: Arc<Resolver>,
    dns_query_cache: Option<Arc<Mutex<DnsQueryCache>>>,
}

pub type SharedContext = Arc<Context>;

impl Context {
    pub fn new(config: Config) -> io::Result<Context> {
        let resolver = create_resolver(config.get_dns_config())?;
        Ok(Context {
            config,
            dns_resolver: Arc::new(resolver),
            dns_query_cache: None,
        })
    }

    pub fn new_dns(config: Config) -> io::Result<Context> {
        let resolver = create_resolver(config.get_dns_config())?;
        Ok(Context {
            config,
            dns_resolver: Arc::new(resolver),
            dns_query_cache: Some(Arc::new(Mutex::new(LruCache::new(1024)))),
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    pub fn dns_resolver(&self) -> &Resolver {
        &*self.dns_resolver
    }

    pub fn dns_query_cache(&self) -> MutexGuard<DnsQueryCache> {
        self.dns_query_cache.as_ref().unwrap().lock().unwrap()
    }
}
