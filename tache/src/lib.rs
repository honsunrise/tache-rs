#![crate_type = "lib"]
#![crate_name = "tache"]
#![recursion_limit = "128"]
#![feature(async_await)]

/// ShadowSocks version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use self::{
    config::{Config, Mode},
    engine::run,
};

// relay::{dns::run as run_dns},

pub mod config;
mod context;
pub(crate) mod dns_resolver;
pub mod engine;
pub mod inbounds;
mod local;
pub mod outbound;
pub mod protocol;
mod utils;
