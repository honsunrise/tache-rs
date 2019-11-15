#![crate_type = "lib"]
#![crate_name = "tache"]
#![recursion_limit = "128"]
#![feature(async_await)]

pub use self::{
    config::{Config, Mode},
    local::run,
};

/// ShadowSocks version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// relay::{dns::run as run_dns},

mod config;
mod context;
mod dns_resolver;
mod inbounds;
mod local;
mod outbound;
mod protocol;
mod rules;
mod utils;
