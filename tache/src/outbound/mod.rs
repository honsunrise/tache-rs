use std::io;
use std::net::SocketAddr;
use tokio::net::{TcpStream, UdpSocket};

mod direct;
mod fallback;
mod socks5;

pub trait Outbound {
    fn name(&self) -> String;
    fn udp(&self) -> bool;
    fn dial(&self, addr: SocketAddr) -> io::Result<TcpStream>;
    fn bind(&self, addr: SocketAddr) -> io::Result<UdpSocket>;
    fn alive(&self) -> bool;
}

pub use direct::Direct;
