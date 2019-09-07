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
use serde::{Deserialize, Serialize};
use serde_urlencoded;
use trust_dns_resolver::config::{NameServerConfigGroup, ResolverConfig};
use url::{self, Url};

use crate::utils::Address;

/// Configuration
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    pub mode: Mode,
    pub log_level: LogLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<ApiConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<DnsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_delay: Option<bool>,
    pub inbounds: Vec<InboundConfig>,
    pub proxies: Vec<ProxyConfig>,
    pub proxy_groups: Vec<ProxyGroupConfig>,
    pub rules: Vec<RuleConfig>,
}

/// Server mode
#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Rule,
    Global,
    Direct,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Mode::Rule => f.write_str("rule"),
            Mode::Global => f.write_str("global"),
            Mode::Direct => f.write_str("direct"),
        }
    }
}

impl FromStr for Mode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rule" => Ok(Mode::Rule),
            "global" => Ok(Mode::Global),
            "direct" => Ok(Mode::Direct),
            _ => Err(()),
        }
    }
}

/// LogLevel
#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
    Silent,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogLevel::Info => f.write_str("info"),
            LogLevel::Warning => f.write_str("warning"),
            LogLevel::Error => f.write_str("error"),
            LogLevel::Debug => f.write_str("debug"),
            LogLevel::Silent => f.write_str("silent"),
        }
    }
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(LogLevel::Info),
            "warning" => Ok(LogLevel::Warning),
            "error" => Ok(LogLevel::Error),
            "debug" => Ok(LogLevel::Debug),
            "silent" => Ok(LogLevel::Silent),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiConfig {
    listen: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_ui: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DnsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    ipv6: Option<bool>,
    listen: Address,
    enhanced_mode: String,
    servers: Vec<String>,
    fallback: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct InboundConfig {
    name: String,
    kind: String,
    listen: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    authentication: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProxyConfig {
    name: String,
    kind: String,
    address: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    udp_timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProxyGroupConfig {
    name: String,
    kind: String,
    proxies: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RuleConfig {
    kind: String,
    source: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Vec<String>>,
    target: String,
    timeout: Option<u64>,
}

/// Configuration parsing error kind
#[derive(Copy, Clone, Debug)]
pub enum ErrorKind {
    MissingField,
    Malformed,
    Invalid,
    JsonParsingError,
    IoError,
}

/// Configuration parsing error
pub struct Error {
    pub kind: ErrorKind,
    pub desc: &'static str,
    pub detail: Option<String>,
}

impl Error {
    pub fn new(kind: ErrorKind, desc: &'static str, detail: Option<String>) -> Error {
        Error { kind, desc, detail }
    }
}

macro_rules! impl_from {
    ($error:ty, $kind:expr, $desc:expr) => {
        impl From<$error> for Error {
            fn from(err: $error) -> Self {
                Error::new($kind, $desc, Some(format!("{:?}", err)))
            }
        }
    };
}

impl_from!(
    ::std::io::Error,
    ErrorKind::IoError,
    "error while reading file"
);
impl_from!(
    json5::Error,
    ErrorKind::JsonParsingError,
    "json parse error"
);

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.detail {
            None => write!(f, "{}", self.desc),
            Some(ref det) => write!(f, "{} {}", self.desc, det),
        }
    }
}

impl Config {
    /// Creates an empty configuration
    pub fn new() -> Config {
        Default::default()
    }

    fn check_valid(&self) -> Result<(), Error> {
        //        let check_local = match config_type {
        //            ConfigType::Local => true,
        //            ConfigType::Server => false,
        //        };
        //
        //        if check_local && (config.local_address.is_none() || config.local_port.is_none()) {
        //            let err = Error::new(
        //                ErrorKind::Malformed,
        //                "`local_address` and `local_port` are required in client",
        //                None,
        //            );
        //            return Err(err);
        //        }
        //
        //        let mut nconfig = Config::new(config_type);
        //
        //        // Standard config
        //        // Client
        //        if let Some(la) = config.local_address {
        //            let port = config.local_port.unwrap();
        //
        //            let local = match la.parse::<Ipv4Addr>() {
        //                Ok(v4) => SocketAddr::V4(SocketAddrV4::new(v4, port)),
        //                Err(..) => match la.parse::<Ipv6Addr>() {
        //                    Ok(v6) => SocketAddr::V6(SocketAddrV6::new(v6, port, 0, 0)),
        //                    Err(..) => {
        //                        let err = Error::new(
        //                            ErrorKind::Malformed,
        //                            "`local_address` must be an ipv4 or ipv6 address",
        //                            None,
        //                        );
        //                        return Err(err);
        //                    }
        //                },
        //            };
        //
        //            nconfig.local = Some(local);
        //        }
        //
        //        // Standard config
        //        // Server
        //        match (config.server, config.server_port, config.password) {
        //            (Some(address), Some(port), Some(pwd)) => {
        //                let addr = match address.parse::<Ipv4Addr>() {
        //                    Ok(v4) => ServerAddr::SocketAddr(SocketAddr::V4(SocketAddrV4::new(v4, port))),
        //                    Err(..) => match address.parse::<Ipv6Addr>() {
        //                        Ok(v6) => ServerAddr::SocketAddr(SocketAddr::V6(SocketAddrV6::new(
        //                            v6, port, 0, 0,
        //                        ))),
        //                        Err(..) => ServerAddr::DomainName(address, port),
        //                    },
        //                };
        //
        //                let timeout = config.timeout.map(Duration::from_secs);
        //                let udp_timeout = config.udp_timeout.map(Duration::from_secs);
        //
        //                let mut nsvr = ServerConfig::new(addr, pwd, timeout);
        //
        //                nsvr.udp_timeout = udp_timeout;
        //
        //                nconfig.server.push(nsvr);
        //            }
        //            (None, None, None) => (),
        //            _ => {
        //                let err = Error::new(
        //                    ErrorKind::Malformed,
        //                    "`server`, `server_port`, `method`, `password` must be provided together",
        //                    None,
        //                );
        //                return Err(err);
        //            }
        //        }
        //
        //        // Ext servers
        //        if let Some(servers) = config.servers {
        //            for svr in servers {
        //                let addr = match svr.address.parse::<Ipv4Addr>() {
        //                    Ok(v4) => {
        //                        ServerAddr::SocketAddr(SocketAddr::V4(SocketAddrV4::new(v4, svr.port)))
        //                    }
        //                    Err(..) => match svr.address.parse::<Ipv6Addr>() {
        //                        Ok(v6) => ServerAddr::SocketAddr(SocketAddr::V6(SocketAddrV6::new(
        //                            v6, svr.port, 0, 0,
        //                        ))),
        //                        Err(..) => ServerAddr::DomainName(svr.address, svr.port),
        //                    },
        //                };
        //
        //                let timeout = svr.timeout.map(Duration::from_secs);
        //                let udp_timeout = config.udp_timeout.map(Duration::from_secs);
        //
        //                let mut nsvr = ServerConfig::new(addr, svr.password, timeout);
        //
        //                nsvr.udp_timeout = udp_timeout;
        //
        //                nconfig.server.push(nsvr);
        //            }
        //        }
        //
        //        // Forbidden IPs
        //        if let Some(forbidden_ip) = config.forbidden_ip {
        //            for fi in forbidden_ip {
        //                match fi.parse::<IpAddr>() {
        //                    Ok(i) => {
        //                        nconfig.forbidden_ip.insert(i);
        //                    }
        //                    Err(err) => {
        //                        error!("Invalid forbidden_ip \"{}\", err: {}", fi, err);
        //                    }
        //                }
        //            }
        //        }
        //
        //        // DNS
        //        nconfig.dns = config.dns;
        //
        //        if let Some(rdns) = config.remote_dns {
        //            match rdns.parse::<SocketAddr>() {
        //                Ok(r) => nconfig.remote_dns = Some(r),
        //                Err(..) => {
        //                    let e = Error::new(
        //                        ErrorKind::Malformed,
        //                        "malformed `remote_dns`, which must be a valid SocketAddr",
        //                        None,
        //                    );
        //                    return Err(e);
        //                }
        //            }
        //        }
        //
        //        // Mode
        //        if let Some(m) = config.mode {
        //            match m.parse::<Mode>() {
        //                Ok(xm) => nconfig.mode = xm,
        //                Err(..) => {
        //                    let e = Error::new(
        //                        ErrorKind::Malformed,
        //                        "malformed `mode`, must be one of `tcp_only`, `udp_only` and `tcp_and_udp`",
        //                        None,
        //                    );
        //                    return Err(e);
        //                }
        //            }
        //        }
        //
        //        // TCP nodelay
        //        if let Some(b) = config.no_delay {
        //            nconfig.no_delay = b;
        //        }

        Ok(())
    }

    pub fn load_from_str(s: &str) -> Result<Config, Error> {
        let c = json5::from_str::<Config>(s)?;
        c.check_valid()?;
        Ok(c)
    }

    pub fn load_from_file(filename: &str) -> Result<Config, Error> {
        let mut reader = OpenOptions::new().read(true).open(&Path::new(filename))?;
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        Config::load_from_str(&content[..])
    }

    pub fn get_dns_config(&self) -> Option<ResolverConfig> {
        self.dns.as_ref().and_then(|ds| {
            match &ds[..] {
                "google" => Some(ResolverConfig::google()),

                "cloudflare" => Some(ResolverConfig::cloudflare()),
                "cloudflare_tls" => Some(ResolverConfig::cloudflare_tls()),
                "cloudflare_https" => Some(ResolverConfig::cloudflare_https()),

                "quad9" => Some(ResolverConfig::quad9()),
                "quad9_tls" => Some(ResolverConfig::quad9_tls()),

                _ => {
                    // Set ips directly
                    match ds.parse::<IpAddr>() {
                        Ok(ip) => Some(ResolverConfig::from_parts(
                            None,
                            vec![],
                            NameServerConfigGroup::from_ips_clear(&[ip], 53),
                        )),
                        Err(..) => {
                            error!(
                                "Failed to parse DNS \"{}\" in config to IpAddr, fallback to system config",
                                ds
                            );
                            None
                        }
                    }
                }
            }
        })
    }

    pub fn get_remote_dns(&self) -> SocketAddr {
        match self.remote_dns {
            None => SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 53)),
            Some(ip) => ip,
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)?
    }
}
