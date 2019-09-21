#![crate_type = "lib"]
#![crate_name = "tache"]
#![recursion_limit = "128"]
#![feature(async_await)]

/// ShadowSocks version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use self::config::*;

// relay::{dns::run as run_dns},

mod config;
mod context;
mod dns_resolver;
pub mod inbounds;
mod outbound;
mod protocol;
pub mod rules;
mod utils;
