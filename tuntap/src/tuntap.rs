use std::io::{Read, Write};
use std::net::Ipv4Addr;

pub trait Tuntap: Read + Write {
    /// Turn on or off the interface.
    fn enabled(&mut self, value: bool) -> Result<()>;

    /// Get the device name.
    fn name(&self) -> &str;

    /// Set the device name.
    fn set_name(&mut self, name: &str) -> Result<()>;

    /// Get the address.
    fn address(&self) -> Result<Ipv4Addr>;

    /// Set the address.
    fn set_address(&mut self, value: Ipv4Addr) -> Result<()>;

    /// Get the destination address.
    fn destination(&self) -> Result<Ipv4Addr>;

    /// Set the destination address.
    fn set_destination(&mut self, value: Ipv4Addr) -> Result<()>;

    /// Get the broadcast address.
    fn broadcast(&self) -> Result<Ipv4Addr>;

    /// Set the broadcast address.
    fn set_broadcast(&mut self, value: Ipv4Addr) -> Result<()>;

    /// Get the netmask.
    fn netmask(&self) -> Result<Ipv4Addr>;

    /// Set the netmask.
    fn set_netmask(&mut self, value: Ipv4Addr) -> Result<()>;

    /// Get the MTU.
    fn mtu(&self) -> Result<i32>;

    /// Set the MTU.
    fn set_mtu(&mut self, value: i32) -> Result<()>;
}
