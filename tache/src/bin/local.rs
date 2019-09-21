//! This is a binary running in the local environment
//!
//! You have to provide all needed configuration attributes via command line parameters,
//! or you could specify a configuration file. The format of configuration file is defined
//! in mod `config`.

use actix::prelude::*;
use clap::{App, Arg};
use futures::{future::Either, prelude::*};
use log::{debug, error, info};
use std::io;
use std::{future::Future, io::Result, net::SocketAddr, process};
use tokio::prelude::*;

use tache::{inbounds::http, rules, Config, InboundConfig, Mode};

mod logging;

fn main() {
    let matches = App::new("tache")
        .version(tache::VERSION)
        .about("A fast tunnel protocol that helps you bypass firewalls.")
        .arg(
            Arg::with_name("VERBOSE")
                .short("v")
                .multiple(true)
                .help("Set the level of debug"),
        )
        .arg(
            Arg::with_name("CONFIG")
                .short("c")
                .long("config")
                .takes_value(true)
                .help("Specify config file"),
        )
        .get_matches();

    let debug_level = matches.occurrences_of("VERBOSE");

    logging::init(true, debug_level, "tache");

    let mut config = match matches.value_of("CONFIG") {
        Some(config_path) => match Config::load_from_file(config_path) {
            Ok(cfg) => cfg,
            Err(err) => {
                error!("{:?}", err);
                return;
            }
        },
        None => Config::new(),
    };

    info!("Tache {}", tache::VERSION);

    debug!("Config: {:?}", config);

    match launch_server(config) {
        Ok(()) => {}
        Err(err) => {
            error!("Server exited unexpectly with error: {}", err);
            process::exit(1);
        }
    }
}

fn launch_server(config: Config) -> Result<()> {
    let system = actix::System::new("local");

    //    let mut proxies = Arc::new(HashMap::new());
    //    // setup proxies
    //    for protocol in config.proxies.iter() {
    //        match protocol {
    //            ProxyConfig::Shadowsocks { name, address, cipher, password, udp } => {
    //                tokio::spawn(async move {});
    //            }
    //            ProxyConfig::VMESS { name, address, uuid, alter_id, cipher, tls } => {
    //                tokio::spawn(async move {});
    //            }
    //            ProxyConfig::Socks5 { name, address, username, password, tls, skip_cert_verify } => {
    //                // build protocol
    //
    //                // run protocol
    //                tokio::spawn(async move {});
    //            }
    //            ProxyConfig::HTTP { name, address, username, password, tls, skip_cert_verify } => {
    //                tokio::spawn(async move {});
    //            }
    //        };
    //    }

    // setup rules
    let modes = rules::build_modes(&config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.description()))?;

    // setup inbounds
    for inbound in config.inbounds.iter() {
        match inbound {
            InboundConfig::HTTP {
                name: _,
                listen,
                authentication: _,
            } => {
                http::setup_http_inbounds()?;
            }
            InboundConfig::Socks5 {
                name: _,
                listen,
                authentication: _,
            } => {}
            InboundConfig::Redir {
                name: _,
                listen,
                authentication: _,
            } => {}
            InboundConfig::TUN { name: _ } => {}
        };
    }

    system.run()
    //    let runtime = Runtime::new().expect("Creating runtime");
    //
    //    let abort_signal = signal::ctrl_c()?;
    //
    //    let result = runtime.block_on(select(
    //        Box::pin(run(config)),
    //        Box::pin(abort_signal.into_future()),
    //    ));
    //
    //    runtime.shutdown_now();
    //
    //    match result {
    //        // Server future resolved without an error. This should never happen.
    //        Either::Left(_) => panic!("Server exited unexpectedly"),
    //        // The abort signal future resolved. Means we should just exit.
    //        Either::Right(..) => Ok(()),
    //    }
}
