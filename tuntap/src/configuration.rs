use std::net::{Ipv4Addr};

use address::IntoAddress;
use platform;

/// Configuration builder for a TUN interface.
#[derive(Clone, Default, Debug)]
pub struct Configuration {
    pub(crate) name:     Option<String>,
    pub(crate) platform: platform::Configuration,

    pub(crate) address:     Option<Ipv4Addr>,
    pub(crate) destination: Option<Ipv4Addr>,
    pub(crate) broadcast:   Option<Ipv4Addr>,
    pub(crate) netmask:     Option<Ipv4Addr>,
    pub(crate) mtu:         Option<i32>,
    pub(crate) enabled:     Option<bool>,
}

impl Configuration {
    /// Access the platform dependant configuration.
    pub fn platform<F>(&mut self, f: F) -> &mut Self
        where F: FnOnce(&mut platform::Configuration) {
        f(&mut self.platform);
        self
    }

    /// Set the name.
    pub fn interfaceName<S: AsRef<str>>(&mut self, name: S) -> &mut Self {
        self.name = Some(name.as_ref().into());
        self
    }

    /// Set the address.
    pub fn address<A: IntoAddress>(&mut self, value: A) -> &mut Self {
        self.address = Some(value.into_address().unwrap());
        self
    }

    /// Set the destination address.
    pub fn destination<A: IntoAddress>(&mut self, value: A) -> &mut Self {
        self.destination = Some(value.into_address().unwrap());
        self
    }

    /// Set the broadcast address.
    pub fn broadcast<A: IntoAddress>(&mut self, value: A) -> &mut Self {
        self.broadcast = Some(value.into_address().unwrap());
        self
    }

    /// Set the netmask.
    pub fn netmask<A: IntoAddress>(&mut self, value: A) -> &mut Self {
        self.netmask = Some(value.into_address().unwrap());
        self
    }

    /// Set the MTU.
    pub fn mtu(&mut self, value: i32) -> &mut Self {
        self.mtu = Some(value);
        self
    }

    /// Set the interface to be enabled once created.
    pub fn up(&mut self) -> &mut Self {
        self.enabled = Some(true);
        self
    }

    /// Set the interface to be disabled once created.
    pub fn down(&mut self) -> &mut Self {
        self.enabled = Some(false);
        self
    }
}