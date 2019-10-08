use log::{debug, error, info};
use bytes::BytesMut;
use futures::future::{select_all, BoxFuture, select, Either};
use http::{header::HeaderValue, Request, Response, StatusCode};
use serde::Serialize;
use std::{env, error::Error, fmt::{self, Display}, io};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::{
    prelude::*,
    codec::{Decoder, Encoder, Framed},
    net::{TcpListener, TcpStream},
};

use crate::{
    config::{Config, InboundConfig},
    context::{Context, SharedContext},
};

use crate::outbound::{self, Outbound};
use std::net::{ToSocketAddrs, SocketAddr};
use crate::config::ProxyConfig;
use crate::protocol;
use crate::rules;
use tokio::io::BufReader;
use crate::rules::{lookup, build_modes};
use trust_dns_resolver::proto::error::ProtoErrorKind::NoError;

fn build_connection_meta<T>(stream: &TcpStream, request: &Request<T>)
                            -> Result<rules::ConnectionMeta, Box<dyn Error>> {
    let host = match request.uri().host() {
        Some(host) => host,
        None => {
            return Err(From::from("not have host"));
        }
    };

    let dst_addr = match host.to_socket_addrs() {
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
    let mut incoming = TcpListener::bind(&listen_address).await?.incoming();
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(mut inbound)) = incoming.next().await {
        let modes = modes.clone();
        let proxies = proxies.clone();
        tokio::spawn(async move {
            //let mut transport = Framed::new(inbound, protocol::Http);
            let mut inbound = BufReader::new(inbound);
            let result = protocol::read_http(&mut inbound).await;

            let request = match result {
                Ok(r) => r,
                Err(e) => {
                    println!("failed to process request {}", e);
                    return;
                }
            };

            let connection_meta = match build_connection_meta(inbound.get_ref(), &request) {
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
            let mut outbound = match outbound.dial(connection_meta.dst_addr.unwrap()).await {
                Ok(r) => r,
                Err(e) => {
                    println!("failed to dial to dst address {}", e);
                    return;
                }
            };

            let (mut lr, mut lw) = inbound.get_mut().split();
            let (mut rr, mut rw) = outbound.split();

            match select(lr.copy(&mut rw), rr.copy(&mut lw)).await {
                Either::Left(r) | Either::Right(r) => {

                },
            };
        });
    }
    Ok(())
}

async fn single_run_socks(listen_address: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut incoming = TcpListener::bind(&listen_address).await?.incoming();
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(inbound)) = incoming.next().await {}
    Ok(())
}

async fn single_run_redir(listen_address: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut incoming = TcpListener::bind(&listen_address).await?.incoming();
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(inbound)) = incoming.next().await {}
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
                tokio::spawn(async move {});
            }
            ProxyConfig::VMESS { name, address, uuid, alter_id, cipher, tls } => {
                tokio::spawn(async move {});
            }
            ProxyConfig::Socks5 { name, address, username, password, tls, skip_cert_verify } => {
                // build protocol

                // run protocol
                tokio::spawn(async move {});
            }
            ProxyConfig::HTTP { name, address, username, password, tls, skip_cert_verify } => {
                tokio::spawn(async move {});
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
                for addr in listen.to_socket_addrs()? {
                    let fut = single_run_http(addr, modes.clone(), proxies.clone());
                    vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
                }
            }
            InboundConfig::Socks5 { name: _, listen, authentication: _ } => {
                for addr in listen.to_socket_addrs()? {
                    let fut = single_run_socks(addr);
                    vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
                }
            }
            InboundConfig::Redir { name: _, listen, authentication: _ } => {
                for addr in listen.to_socket_addrs()? {
                    let fut = single_run_redir(addr);
                    vf.push(Box::pin(fut) as BoxFuture<Result<(), Box<dyn Error>>>);
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