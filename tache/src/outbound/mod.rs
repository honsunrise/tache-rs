use std::io;

use async_trait::async_trait;
use async_std::net::{TcpStream, UdpSocket, SocketAddr};
pub use direct::Direct;

mod direct;
mod fallback;
mod socks5;

#[async_trait]
pub trait Outbound {
    fn name(&self) -> String;
    async fn udp(&self) -> bool;
    async fn dial(&self, addr: SocketAddr) -> io::Result<TcpStream>;
    async fn bind(&self, addr: SocketAddr) -> io::Result<UdpSocket>;
    async fn alive(&self) -> bool;
}
