//! shadowsocks is a fast tunnel proxy that helps you bypass firewalls.
//!
//! Currently it supports SOCKS5 and HTTP Proxy protocol.
//!
//! ## Usage
//!
//! Build shadowsocks and you will get at least 2 binaries: `sslocal` and `ssserver`
//!
//! Write your servers in a configuration file. Format is defined in
//! [shadowsocks' documentation](https://github.com/shadowsocks/shadowsocks/wiki)
//!
//! For example:
//!
//! ```json
//! {
//!    "server": "my_server_ip",
//!    "server_port": 8388,
//!    "local_address": "127.0.0.1",
//!    "local_port": 1080,
//!    "password": "mypassword",
//!    "timeout": 300,
//!    "method": "aes-256-cfb"
//! }
//! ```
//!
//! Save it in file `shadowsocks.json` and run local proxy server with
//!
//! ```bash
//! cargo run --bin sslocal -- -c shadowsocks.json
//! ```
//!
//! Now you can use SOCKS5 protocol to proxy your requests, for example:
//!
//! ```bash
//! curl --socks5-hostname 127.0.0.1:1080 https://www.google.com
//! ```
//!
//! On the server side, you can run the server with
//!
//! ```bash
//! cargo run --bin ssserver -- -c shadowsocks.json
//! ```
//!
//! Server should use the same configuration file as local, except the listen addresses for servers must be socket
//! addresses.
//!
//! Of course, you can also use `cargo install` to install binaries.
//!
//! ## API Usage
//!
//! Example to write a local server
//!
//! ```no_run
//! use shadowsocks::{run_local, Config, ConfigType};
//!
//! let config = Config::load_from_file("shadowsocks.json", ConfigType::Local).unwrap();
//! run_local(config);
//! ```
//!
//! That's all! And let me show you how to run a proxy server
//!
//! ```no_run
//! use shadowsocks::{run_server, Config, ConfigType};
//!
//! let config = Config::load_from_file("shadowsocks.json", ConfigType::Server).unwrap();
//! run_server(config);
//! ```

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
pub mod engine;
pub mod outbound;
pub mod proxy;
mod utils;
