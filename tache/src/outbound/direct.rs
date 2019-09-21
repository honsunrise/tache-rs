use crate::outbound::Outbound;
use std::io;
use std::iter::Iterator;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, TcpStream, UdpSocket};
use std::time::Duration;

pub struct Direct {
    pub name: String,
}

impl Direct
{
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn udp(&self) -> bool {
        true
    }

    pub async fn dial<T>(&self, addr: T) -> io::Result<TcpStream>
        where
            T: ToSocketAddrs,
    {
        let mut socket_addr = addr.to_socket_addrs()?;
        let addr = match socket_addr.next() {
            Some(addr) => addr,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "no socket addresses could be resolved",
                ));
            }
        };
//        if !socket_addr.next().is_none() {
//            Err(io::Error::new(
//                io::ErrorKind::Other,
//                "more than one address resolved",
//            ))
//        }
        let mut stream = TcpStream::connect(addr)?;
        Ok(stream)
    }

    pub async fn bind<T>(&self, addr: T) -> io::Result<UdpSocket>
        where
            T: ToSocketAddrs,
    {
        let mut socket_addr = addr.to_socket_addrs()?;
        let addr = match socket_addr.next() {
            Some(addr) => addr,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "no socket addresses could be resolved",
                ));
            }
        };
//        if !socket_addr.next().is_none() {
//            Err(io::Error::new(
//                io::ErrorKind::Other,
//                "more than one address resolved",
//            ))
//        }
        let local_addr = SocketAddr::new(IpAddr::from(Ipv4Addr::new(0, 0, 0, 0)), 0);
        let mut remote_udp = UdpSocket::bind(&local_addr)?;
        remote_udp.connect(&addr)?;
        Ok(remote_udp)
    }

    fn alive(&self) -> bool {
        true
    }
}
