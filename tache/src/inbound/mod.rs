use async_std::net::TcpStream;
use std::io::{self, BufReader, Error, ErrorKind};

mod http_s;
mod socket;
mod tun;

pub struct ConnectionMeta {}

pub(crate) trait InboundMeta {
    fn build_meta() -> Option<ConnectionMeta>;
}

pub trait Inbound {
    fn listen(&self) -> io::Result<(ConnectionMeta, TcpStream)> {
        Err(Error::from(ErrorKind::NotConnected))
    }
}