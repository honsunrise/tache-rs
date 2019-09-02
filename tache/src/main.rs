mod config;
mod engine;
mod inbound;
mod metrics;
mod outbound;
mod proxy;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

use async_std::prelude::*;
use async_std::task;

use clap::{App, Arg, SubCommand};

use std::fs::File;
use std::io::{self, Write};
use std::net::UdpSocket;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use std::sync::Mutex;

use crate::config::Config;
use crate::engine::Engine;

const VERSION: u8 = 1;

struct DualLogger {
    file: Mutex<Option<File>>,
}

impl DualLogger {
    pub fn new<P: AsRef<Path>>(path: Option<P>) -> Result<Self, io::Error> {
        if let Some(path) = path {
            let file = File::create(path)?;
            Ok(DualLogger {
                file: Mutex::new(Some(file)),
            })
        } else {
            Ok(DualLogger {
                file: Mutex::new(None),
            })
        }
    }
}

impl log::Log for DualLogger {
    #[inline]
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    #[inline]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
            let mut file = self.file.lock().expect("Lock poisoned");
            if let Some(ref mut file) = *file {
                let time =
                    time::strftime("%F %T", &time::now()).expect("Failed to format timestamp");
                writeln!(file, "{} - {} - {}", time, record.level(), record.args())
                    .expect("Failed to write to logfile");
            }
        }
    }

    #[inline]
    fn flush(&self) {
        let mut file = self.file.lock().expect("Lock poisoned");
        if let Some(ref mut file) = *file {
            file.flush().unwrap()
        }
    }
}

fn run_script(script: &str, ifname: &str) {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&script).env("IFNAME", ifname);
    debug!("Running script: {:?}", cmd);
    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                error!("Script returned with error: {:?}", status.code())
            }
        }
        Err(e) => error!("Failed to execute script {:?}: {}", script, e),
    }
}

async fn run(config: Config) {
    // start script

    // start engine
    let engine_config = engine::Config{modes: "global"};
    let engine = Engine::new(&engine_config);
    engine.run().await;
    // stop script
}

fn main() {
    let matches = App::new("Tache")
        .version("1.0")
        .author("Tache Team")
        .about("Rule base proxy")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    // setup logger
    let log_filename = matches.value_of("log");
    let logger = DualLogger::new(log_filename).unwrap();
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(match matches.occurrences_of("v") {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 | _ => log::LevelFilter::Debug,
    });

    // build config
    let mut config = Config::default();
    let filename = matches.value_of("config").unwrap_or("default.yaml");
    let f = File::open(filename).unwrap();
    let config_file = serde_yaml::from_reader(f).unwrap();
    config.merge_file(config_file);
    task::block_on(run(config))
}
