use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;

use std::io::{self, BufReader, Error, ErrorKind};

#[macro_use]
use log;
use crate::inbound::{Inbound, ConnectionMeta};

#[derive(Default, Debug)]
pub struct Config<'a> {
    address: Option<&'a str>,
}

pub struct HttpInbound {
    listener: TcpListener,
}

impl HttpInbound {
    pub async fn new(config: Config<'_>) -> HttpInbound {
        let listen_address = config.address.unwrap();
        let listener = TcpListener::bind(listen_address).await.unwrap();
        let inbound = HttpInbound { listener };

        inbound
    }
}

impl Inbound for HttpInbound {
    fn listen(&self) -> io::Result<(super::ConnectionMeta, TcpStream)> {
        Err(Error::from(ErrorKind::NotConnected))
    }
}
