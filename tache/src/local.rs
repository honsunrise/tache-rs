use log::{debug, error, info};
use bytes::BytesMut;
use futures::{
    SinkExt,
    StreamExt,
    future::{select_all, BoxFuture},
};
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

use crate::outbound::Outbound;
use std::net::{ToSocketAddrs, SocketAddr};
use crate::config::ProxyConfig;
use crate::protocol;
use crate::rules;

async fn build_connection_meta<T> (stream: &TcpStream, request: &Request<T>)
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

async fn pipe(request: Request<()>, inbound: &TcpStream, outbound: &TcpStream)
              -> Result<(), Box<dyn Error>> {
    Ok(())
}

async fn single_run_http(listen_address: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut incoming = TcpListener::bind(&listen_address).await?.incoming();
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(inbound)) = incoming.next().await {
        tokio::spawn(async move {
            let mut transport = Framed::new(inbound, protocol::Http);

            while let Some(request) = transport.next().await {
                let request = match request {
                    Ok(r) => r,
                    Err(e) => {
                        println!("failed to process request {}", e);
                        return;
                    }
                };

                let connection_meta = match build_connection_meta(
                    transport.get_ref(), &request).await {
                    Ok(r) => r,
                    Err(e) => {
                        println!("failed to process request {}", e);
                        return;
                    }
                };

                info!("Connection meta: {:?}", connection_meta);

//                let outbound = match run_rule(
//                    transport.get_ref(), connection_meta).await {
//                    Ok(r) => r,
//                    Err(e) => {
//                        println!("failed to process request {}", e);
//                        return;
//                    }
//                };
//
//                if let Err(e) = pipe(
//                    request, transport.get_ref(), outbound).await {
//                    println!("failed to process request {}", e);
//                    return;
//                }
            }
        });
    }
    Ok(())
}

async fn single_run_socks(listen_address: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut incoming = TcpListener::bind(&listen_address).await?.incoming();
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(inbound)) = incoming.next().await {
    }
    Ok(())
}

async fn single_run_redir(listen_address: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut incoming = TcpListener::bind(&listen_address).await?.incoming();
    println!("Listening on: {}", &listen_address);

    while let Some(Ok(inbound)) = incoming.next().await {
    }
    Ok(())
}

async fn single_run_tun() -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub async fn run(config: Config) -> io::Result<()> {
//    let mut proxies = Arc::new(HashMap::new());
//    // setup proxies
//    for protocol in config.proxies.iter() {
//        match protocol {
//            ProxyConfig::Shadowsocks { name, address, cipher, password, udp } => {
//                tokio::spawn(async move {});
//            }
//            ProxyConfig::VMESS { name, address, uuid, alter_id, cipher, tls } => {
//                tokio::spawn(async move {});
//            }
//            ProxyConfig::Socks5 { name, address, username, password, tls, skip_cert_verify } => {
//                // build protocol
//
//                // run protocol
//                tokio::spawn(async move {});
//            }
//            ProxyConfig::HTTP { name, address, username, password, tls, skip_cert_verify } => {
//                tokio::spawn(async move {});
//            }
//        };
//    }

    // setup rules

    let mut vf = Vec::new();
    // setup inbounds
    for inbound in config.inbounds.iter() {
        match inbound {
            InboundConfig::HTTP { name: _, listen, authentication: _ } => {
                for addr in listen.to_socket_addrs()? {
                    let fut = single_run_http(addr);
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