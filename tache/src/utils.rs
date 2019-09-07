use std::{
    collections::HashSet,
    convert::From,
    default::Default,
    error,
    fmt::{self, Debug, Display, Formatter},
    fs::OpenOptions,
    io::Read,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    option::Option,
    path::Path,
    str::FromStr,
    string::ToString,
    time::Duration,
};

use base64::{decode_config, encode_config, URL_SAFE_NO_PAD};
use bytes::Bytes;
use json5;
use log::{error, trace};
use serde::{
    de::{self, Deserialize, Deserializer, Visitor},
    ser::{self, Serialize, Serializer},
    *,
};
use serde_urlencoded;
use trust_dns_resolver::config::{NameServerConfigGroup, ResolverConfig};
use url::{self, Url};

type DomainName = (String, u16);

#[derive(Debug)]
pub struct DomainNameError;

impl FromStr for DomainName {
    type Err = DomainNameError;

    fn from_str(s: &str) -> Result<DomainName, DomainNameError> {
        let mut sp = s.split(':');
        match (sp.next(), sp.next()) {
            (Some(dn), Some(port)) => match port.parse::<u16>() {
                Ok(port) => Ok((dn.to_owned(), port)),
                Err(..) => Err(DomainNameError),
            },
            _ => Err(DomainNameError),
        }
    }
}

impl Display for DomainName {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.0, self.1)
    }
}

impl<'de> Deserialize<'de> for DomainName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            struct DomainNameVisitor;

            impl<'de> Visitor<'de> for DomainNameVisitor {
                type Value = DomainName;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("DomainName address")
                }

                fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    s.parse().map_err(de::Error::custom)
                }
            }

            deserializer.deserialize_str(DomainNameVisitor)
        } else {
            <(String, u16)>::deserialize(deserializer)
                .map(|(ip, port)| -> DomainName { (ip, port) })
        }
    }
}

impl Serialize for DomainName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let mut output = String::new();
            write!(output, "{}:{}", self.0, self.1)?;
            serializer.serialize_str(&output)
        } else {
            (self.0, self.1).serialize(serializer)
        }
    }
}

/// Address
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Address {
    /// IP Address
    SocketAddr(SocketAddr),
    /// Domain name address, eg. example.com:8080
    DomainName(DomainName),
}

impl Address {
    /// Get address for server listener
    /// Panic if address is domain name
    pub fn listen_addr(&self) -> &SocketAddr {
        match *self {
            Address::SocketAddr(ref s) => s,
            _ => panic!("Cannot use domain name as server listen address"),
        }
    }

    /// Get string representation of domain
    pub fn host(&self) -> String {
        match *self {
            Address::SocketAddr(ref s) => s.ip().to_string(),
            Address::DomainName(ref dm) => dm.clone(),
        }
    }

    /// Get port
    pub fn port(&self) -> u16 {
        match *self {
            Address::SocketAddr(ref s) => s.port(),
            Address::DomainName(ref p) => p.1,
        }
    }
}

/// Parse `Address` error
#[derive(Debug)]
pub struct AddressError;

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Address, AddressError> {
        match s.parse::<SocketAddr>() {
            Ok(addr) => Ok(Address::SocketAddr(addr)),
            Err(..) => {
                let mut sp = s.split(':');
                match (sp.next(), sp.next()) {
                    (Some(dn), Some(port)) => match port.parse::<u16>() {
                        Ok(port) => Ok(Address::DomainName((dn.to_owned(), port))),
                        Err(..) => Err(AddressError),
                    },
                    _ => Err(AddressError),
                }
            }
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}
