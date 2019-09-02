mod rules;

use std::collections::HashMap;
use std::net::{TcpStream, UdpSocket};
use async_std::prelude::*;
use async_std::{io, task};

use crate::inbound::Inbound;
use crate::outbound::Outbound;
use std::net::Shutdown::Read;
use std::sync::Arc;


pub struct MODE(Vec<Box<dyn rules::Rule>>);

#[derive(Clone, Default, Debug)]
pub struct Config<'a> {
    pub modes: &'a str,
}

pub enum Transport {
    UDP(UdpSocket),
    TCP(TcpStream),
}

pub struct InboundWarp {
    conn: Transport,
}

pub struct OutboundWarp {
    conn: Transport,
}

pub struct Engine {
    inbounds: Vec<Arc<dyn Inbound + Send + Sync>>,
    outbounds: Vec<Box<dyn Outbound>>,
    modes: HashMap<Box<str>, MODE>,
}

impl Engine {
    #[inline]
    pub fn new(config: &Config) -> Engine {
        let modes = HashMap::new();

        Engine {
            modes,
            inbounds: vec![],
            outbounds: vec![]
        }
    }

    pub fn get_modes(&self) -> Vec<&str> {
        self.modes.keys().map(|key|key.as_ref()).collect()
    }

    pub fn update_config(config: &Config) -> Result<(), &'static str> {
        Err("not implement")
    }

    pub fn lookup(&self) {}

    pub fn run(&self) {
        for inbound in self.inbounds.iter() {
            let inbound  = inbound.clone();
            task::spawn(async move {
                loop {
                    let mut socket = match inbound.listen() {
                        Ok((_, socket)) => socket,
                        Err(e) => {
                            return
                        }
                    };

                    task::spawn(async move {
                        let mut buf = [0; 1024];

                        // In a loop, read data from the socket and write the data back.
                        loop {
                            let n = match socket.read(&mut buf).await {
                                // socket closed
                                Ok(n) if n == 0 => return,
                                Ok(n) => n,
                                Err(e) => {
                                    println!("failed to read from socket; err = {:?}", e);
                                    return;
                                }
                            };

                            // Write the data back
                            if let Err(e) = socket.write_all(&buf[0..n]).await {
                                println!("failed to write to socket; err = {:?}", e);
                                return;
                            }
                        }
                    });
                }
            });
        }
    }

    fn handler_http_inbound(&self, inbound: InboundWarp, outbound: OutboundWarp) {}

    fn handler_socks_inbound(&self, inbound: InboundWarp, outbound: OutboundWarp) {}
}
