use async_trait::async_trait;
use std::io;
use std::net::SocketAddr;
use tokio::net::{TcpStream, UdpSocket};

mod direct;
mod fallback;
mod socks5;

pub use direct::Direct;

#[async_trait]
pub trait Outbound {
    fn name(&self) -> String;
    async fn udp(&self) -> bool;
    async fn dial(&self, addr: SocketAddr) -> io::Result<TcpStream>;
    async fn bind(&self, addr: SocketAddr) -> io::Result<UdpSocket>;
    async fn alive(&self) -> bool;
}
