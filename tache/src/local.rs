use std::{env, error::Error, fmt::{self, Display}};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_std::{
    io::{self, BufReader},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
    future,
    task,
};
use bytes::BytesMut;
use futures::future::{BoxFuture, Either, select, select_all};
use http::{header::HeaderValue, Request, Response, StatusCode};
use log::{debug, error, info};
use serde::Serialize;
use trust_dns_resolver::proto::error::ProtoErrorKind::NoError;

use crate::{
    config::{Config, InboundConfig},
    context::{Context, SharedContext},
};
use crate::config::ProxyConfig;
use crate::outbound::{self, Outbound};
use crate::protocol;
use crate::rules;
use crate::rules::{build_modes, lookup};
use crate::utils::Address;

async fn build_connection_meta<T>(stream: &TcpStream, request: &Request<T>)
                                  -> Result<rules::ConnectionMeta, Box<dyn Error>> {
    let host = match request.uri().host() {
        Some(host) => host,
        None => {
            return Err(From::from("not have host"));
        }
    };

    let dst_addr = match host.to_socket_addrs().await {
        Ok(mut addrs) => addrs.next(),
        Err(e) => None
    };

    let src_addr = match stream.peer_addr() {
        Ok(addr) => Some(addr),
        Err(e) => None
    };

    Ok(rules::ConnectionMeta {
        udp: false,
        host: String::from(host),
        dst_addr,
        src_addr,
    })
}

async fn single_run_http(listen_address: SocketAddr,
                         modes: HashMap<String, Arc<rules::MODE>>,
                         proxies: HashMap<String, Arc<Box<dyn Outbound + Send + Sync>>>)
                         -> Result<(), Box<dyn Error>> {
    let modes = Arc::new(modes);
    let listen = TcpListener::bind(&listen_address).await?;
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(inbound)) = listen.incoming().next().await {
        let modes = modes.clone();
        let proxies = proxies.clone();
        task::spawn(async move {
            //let mut transport = Framed::new(inbound, protocol::Http);
            let mut reader = BufReader::new(inbound);
            let result = protocol::read_http(&mut reader).await;
            let inbound = reader.get_ref();

            let request = match result {
                Ok(r) => r,
                Err(e) => {
                    println!("failed to process request {}", e);
                    return;
                }
            };

            let connection_meta = match build_connection_meta(inbound, &request).await {
                Ok(r) => r,
                Err(e) => {
                    println!("failed to process request {}", e);
                    return;
                }
            };

            info!("Connection meta: {:?}", connection_meta);

            let outbound = match lookup(modes["DIRECT"].clone(), &connection_meta).await {
                Ok(r) => r,
                Err(e) => {
                    println!("failed to process request {}", e);
                    return;
                }
            };

            info!("Get outbound: {:?}", outbound);

            let outbound = match proxies.get(outbound.as_str()) {
                Some(r) => r,
                None => {
                    println!("failed to get outbound {}", outbound);
                    return;
                }
            };
            let outbound = match outbound.dial(connection_meta.dst_addr.unwrap()).await {
                Ok(r) => r,
                Err(e) => {
                    println!("failed to dial to dst address {}", e);
                    return;
                }
            };

            let (l_reader, l_writer) = &mut (inbound, inbound);
            let (r_reader, r_writer) = &mut (&outbound, &outbound);

            match select(Box::pin(io::copy(l_reader, r_writer)),
                         Box::pin(io::copy(r_reader, l_writer))).await {
                Either::Left(r) | Either::Right(r) => {}
            };
        });
    }
    Ok(())
}

async fn single_run_socks(listen_address: SocketAddr,
                          modes: HashMap<String, Arc<rules::MODE>>,
                          proxies: HashMap<String, Arc<Box<dyn Outbound + Send + Sync>>>)
                          -> Result<(), Box<dyn Error>> {
    let listen = TcpListener::bind(&listen_address).await?;
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(inbound)) = listen.incoming().next().await {
        let modes = modes.clone();
        let proxies = proxies.clone();
        task::spawn(async move {});
    }
    Ok(())
}

async fn single_run_redir(listen_address: SocketAddr) -> Result<(), Box<dyn Error>> {
    let listen = TcpListener::bind(&listen_address).await?;
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(inbound)) = listen.incoming().next().await {}
    Ok(())
}

async fn single_run_tun() -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub async fn run(config: Config) -> io::Result<()> {
    let mut proxies: HashMap<String, Arc<Box<dyn Outbound + Send + Sync>>> = HashMap::new();
    // setup proxies
    for protocol in config.proxies.iter() {
        match protocol {
            ProxyConfig::Shadowsocks { name, address, cipher, password, udp } => {
                task::spawn(async move {});
            }
            ProxyConfig::VMESS { name, address, uuid, alter_id, cipher, tls } => {
                task::spawn(async move {});
            }
            ProxyConfig::Socks5 { name, address, username, password, tls, skip_cert_verify } => {
                // build protocol

                // run protocol
                task::spawn(async move {});
            }
            ProxyConfig::HTTP { name, address, username, password, tls, skip_cert_verify } => {
                task::spawn(async move {});
            }
            ProxyConfig::Direct { name } => {
                proxies.insert(name.to_owned(), Arc::new(Box::new(outbound::Direct::new(name))));
            }
        };
    }

    // setup rules
    let modes = build_modes(&config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.description()))?;

    let mut vf = Vec::new();
    // setup inbounds
    for inbound in config.inbounds.iter() {
        match inbound {
            InboundConfig::HTTP { name: _, listen, authentication: _ } => {
                match listen {
                    Address::SocketAddr(addr) => {
                        for addr in addr.to_socket_addrs().await? {
                            let fut = single_run_http(addr, modes.clone(), proxies.clone());
                            vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
                        }
                    }
                    Address::DomainName(ref domain) => {
                        for addr in (domain.0.as_ref(), domain.1).to_socket_addrs().await? {
                            let fut = single_run_http(addr, modes.clone(), proxies.clone());
                            vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
                        }
                    }
                }
            }
            InboundConfig::Socks5 { name: _, listen, authentication: _ } => {
                match listen {
                    Address::SocketAddr(addr) => {
                        for addr in addr.to_socket_addrs().await? {
                            let fut = single_run_socks(addr, modes.clone(), proxies.clone());
                            vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
                        }
                    }
                    Address::DomainName(ref domain) => {
                        for addr in (domain.0.as_ref(), domain.1).to_socket_addrs().await? {
                            let fut = single_run_socks(addr, modes.clone(), proxies.clone());
                            vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
                        }
                    }
                }
            }
            InboundConfig::Redir { name: _, listen, authentication: _ } => {
                match listen {
                    Address::SocketAddr(addr) => {
                        for addr in addr.to_socket_addrs().await? {
                            let fut = single_run_redir(addr);
                            vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
                        }
                    }
                    Address::DomainName(ref domain) => {
                        for addr in (domain.0.as_ref(), domain.1).to_socket_addrs().await? {
                            let fut = single_run_redir(addr);
                            vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
                        }
                    }
                }
            }
            InboundConfig::TUN { name: _ } => {
                let fut = single_run_tun();
                vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
            }
        };
    }

    let (res, ..) = select_all(vf.into_iter()).await;
    error!("One of inbound exited unexpectedly, result: {:?}", res);
    Err(io::Error::new(io::ErrorKind::Other, "server exited unexpectedly"))
}