use crate::outbound::Outbound;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::time::Duration;
use tokio::net::{TcpStream, UdpSocket};

pub struct Direct {
    name: String,
}

impl<T> Outbound<T> for Direct
where
    T: ToSocketAddrs,
{
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn udp(&self) -> bool {
        true
    }

    fn dial(&self, addr: T) -> io::Result<TcpStream> {
        let socket_addr = addr.to_socket_addrs()?;
        let mut stream = TcpStream::connect(socket_addr).await?;
        stream.set_keepalive(Some(Duration::from_secs(30)))?;
        stream.set_nodelay(true)?;
        Ok(stream)
    }

    fn bind(&self, addr: T) -> io::Result<UdpSocket> {
        let socket_addr = addr.to_socket_addrs()?;
        let local_addr = SocketAddr::new(IpAddr::from(Ipv4Addr::new(0, 0, 0, 0)), 0);
        let mut remote_udp = UdpSocket::bind(&local_addr)?;
        remote_udp.connect(socket_addr)?;
        Ok(remote_udp)
    }

    fn alive(&self) -> bool {
        true;
    }
}
