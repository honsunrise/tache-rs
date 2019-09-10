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

use crate::utils::Address;

/// Configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub mode: Mode,
    pub log_level: LogLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<ApiConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<DNSConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_delay: Option<bool>,
    pub inbounds: Vec<InboundConfig>,
    pub proxies: Vec<ProxyConfig>,
    pub proxy_groups: Vec<ProxyGroupConfig>,
    pub rules: Vec<RuleConfig>,
}

/// Server mode
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
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

impl Default for Mode {
    fn default() -> Self {
        Mode::Direct
    }
}

/// LogLevel
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
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

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiConfig {
    pub listen: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ui: Option<String>,
}

/// DNS Server work mode
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum DNSMode {
    RedirHost,
    FakeIP,
}

impl fmt::Display for DNSMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DNSMode::RedirHost => f.write_str("redir-host"),
            DNSMode::FakeIP => f.write_str("fake-ip"),
        }
    }
}

impl FromStr for DNSMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "redir-host" => Ok(DNSMode::RedirHost),
            "fake-ip" => Ok(DNSMode::FakeIP),
            _ => Err(()),
        }
    }
}

impl Default for DNSMode {
    fn default() -> Self {
        DNSMode::RedirHost
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DNSConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6: Option<bool>,
    pub listen: Address,
    pub mode: DNSMode,
    pub servers: Vec<String>,
    pub fallback: Vec<String>,
}

/// Inbound Kind
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum InboundKind {
    HTTP,
    Socks5,
    Redir,
    TUN,
}

impl fmt::Display for InboundKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InboundKind::HTTP => f.write_str("http"),
            InboundKind::Socks5 => f.write_str("socks5"),
            InboundKind::Redir => f.write_str("redir"),
            InboundKind::TUN => f.write_str("tun"),
        }
    }
}

impl FromStr for InboundKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" => Ok(InboundKind::HTTP),
            "socks5" => Ok(InboundKind::Socks5),
            "redir" => Ok(InboundKind::Redir),
            "tun" => Ok(InboundKind::TUN),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InboundConfig {
    pub name: String,
    pub kind: InboundKind,
    pub listen: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Vec<String>>,
}

/// Inbound Kind
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ProxyKind {
    Shadowsocks,
    VMESS,
    Socks5,
    HTTP,
}

impl fmt::Display for ProxyKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProxyKind::Shadowsocks => f.write_str("shadowsocks"),
            ProxyKind::VMESS => f.write_str("vmess"),
            ProxyKind::Socks5 => f.write_str("socks5"),
            ProxyKind::HTTP => f.write_str("http"),
        }
    }
}

impl FromStr for ProxyKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "shadowsocks" => Ok(ProxyKind::Shadowsocks),
            "vmess" => Ok(ProxyKind::VMESS),
            "socks5" => Ok(ProxyKind::Socks5),
            "http" => Ok(ProxyKind::HTTP),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyConfig {
    pub name: String,
    pub kind: ProxyKind,
    pub address: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp_timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyGroupConfig {
    name: String,
    kind: String,
    proxies: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleConfig {
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
        Config {
            mode: Default::default(),
            log_level: Default::default(),
            api: None,
            dns: None,
            no_delay: None,
            inbounds: vec![],
            proxies: vec![],
            proxy_groups: vec![],
            rules: vec![],
        }
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
        self.dns
            .as_ref()
            .map(|ds| ds.servers.clone())
            .and_then(|servers| {
                let mut result = ResolverConfig::new();
                for address in servers {
                    let group = match &address[..] {
                        "google" => Some(NameServerConfigGroup::google()),

                        "cloudflare" => Some(NameServerConfigGroup::cloudflare()),
                        "cloudflare_tls" => Some(NameServerConfigGroup::cloudflare_tls()),
                        "cloudflare_https" => Some(NameServerConfigGroup::cloudflare_https()),

                        "quad9" => Some(NameServerConfigGroup::quad9()),
                        "quad9_tls" => Some(NameServerConfigGroup::quad9_tls()),

                        _ => {
                            // Set ips directly
                            match address.parse::<IpAddr>() {
                                Ok(ip) => Some(NameServerConfigGroup::from_ips_clear(&[ip], 53)),
                                Err(..) => {
                                    error!(
                                        "Failed to parse DNS \"{}\" in config to IpAddr, \
                                         fallback to system config",
                                        address
                                    );
                                    None
                                }
                            }
                        }
                    };
                    if let Some(config) = group {
                        for name_server in config.iter().cloned() {
                            result.add_name_server(name_server);
                        }
                    }
                }
                Some(result)
            })
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}
