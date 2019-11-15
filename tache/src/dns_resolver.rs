//! Asynchronous DNS resolver

use std::{
    io::{self, ErrorKind},
    net::SocketAddr,
};

use trust_dns_resolver::{config::ResolverConfig, Resolver};

use crate::context::SharedContext;

pub fn create_resolver(dns: Option<ResolverConfig>) -> io::Result<Resolver> {
    let resolver = {
        // To make this independent, if targeting macOS, BSD, Linux, or Windows, we can use the system's configuration:
        #[cfg(any(unix, windows))]
        {
            if let Some(conf) = dns {
                use trust_dns_resolver::config::ResolverOpts;
                Resolver::new(conf, ResolverOpts::default())
            } else {
                use trust_dns_resolver::system_conf::read_system_conf;
                // use the system resolver configuration
                let (config, opts) = read_system_conf().expect("Failed to read global dns sysconf");
                Resolver::new(config, opts)
            }
        }

        // For other operating systems, we can use one of the preconfigured definitions
        #[cfg(not(any(unix, windows)))]
        {
            // Directly reference the config types
            use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};

            if let Some(conf) = dns {
                Resolver::new(conf, ResolverOpts::default())
            } else {
                // Get a new resolver with the google nameservers as the upstream recursive resolvers
                Resolver::new(ResolverConfig::google(), ResolverOpts::default())
            }
        }
    };

    resolver
}

async fn inner_resolve(
    context: SharedContext,
    addr: &str,
    port: u16,
) -> io::Result<Vec<SocketAddr>> {
    // let owned_addr = addr.to_owned();
    match context.dns_resolver().lookup_ip(addr) {
        Err(err) => {
            // error!("Failed to resolve {}, err: {}", owned_addr, err);
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("dns resolve error: {}", err),
            ))
        }
        Ok(lookup_result) => {
            let mut vaddr = Vec::new();
            for ip in lookup_result.iter() {
                vaddr.push(SocketAddr::new(ip, port));
            }

            if vaddr.is_empty() {
                let err = io::Error::new(
                    ErrorKind::Other,
                    // format!("resolved {} to empty address, all IPs are filtered", owned_addr),
                    "resolved to empty address, all IPs are filtered",
                );
                Err(err)
            } else {
                // debug!("Resolved {} => {:?}", owned_addr, vaddr);
                Ok(vaddr)
            }
        }
    }
}

/// Resolve address to IP
pub async fn resolve(context: SharedContext, addr: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
    inner_resolve(context, addr, port).await
}
