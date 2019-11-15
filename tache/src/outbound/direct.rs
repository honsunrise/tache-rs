use async_std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::io;

use async_trait::async_trait;
use net2::TcpStreamExt;

use crate::outbound::Outbound;

pub struct Direct {
    name: String,
}

impl Direct {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
        }
    }
}

#[async_trait]
impl Outbound for Direct {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    async fn udp(&self) -> bool {
        true
    }

    async fn dial(&self, addr: SocketAddr) -> io::Result<TcpStream> {
        let stream = TcpStream::connect(addr).await?;
        //        stream.set_keepalive(Some(Duration::from_secs(30)))?;
        stream.set_nodelay(true)?;
        Ok(stream)
    }

    async fn bind(&self, addr: SocketAddr) -> io::Result<UdpSocket> {
        let local_addr = SocketAddr::new(IpAddr::from(Ipv4Addr::new(0, 0, 0, 0)), 0);
        let remote_udp = UdpSocket::bind(&local_addr).await?;
        remote_udp.connect(addr).await?;
        Ok(remote_udp)
    }

    async fn alive(&self) -> bool {
        true
    }
}
