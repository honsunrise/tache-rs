use std::io;
use std::net::ToSocketAddrs;
use tokio::net::{TcpStream, UdpSocket};

mod direct;
mod fallback;
mod socks5;

pub trait Outbound<T>
where
    T: ToSocketAddrs,
{
    fn name(&self) -> String;
    fn udp(&self) -> bool;
    fn dial(&self, addr: T) -> io::Result<TcpStream>;
    fn bind(&self, addr: T) -> io::Result<UdpSocket>;
    fn alive(&self) -> bool;
}
