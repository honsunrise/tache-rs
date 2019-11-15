//! This is a binary running in the local environment
//!
//! You have to provide all needed configuration attributes via command line parameters,
//! or you could specify a configuration file. The format of configuration file is defined
//! in mod `config`.

use std::{io::Result as IoResult, process};

use clap::{App, Arg};

use log::{debug, error, info};

use tache::{run, Config};

mod logging;
use async_std::task;

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

    logging::init(true, debug_level, "tachelocal");

    let config = match matches.value_of("CONFIG") {
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

fn launch_server(config: Config) -> IoResult<()> {
    task::block_on(Box::pin(run(config)));
    panic!("Server exited unexpectedly");
}
