use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use http::{header::HeaderValue, Request, Response, StatusCode};
use serde::Serialize;
use std::{env, error::Error as StdError, fmt::{self, Display}, io};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    prelude::*,
    codec::{Decoder, Encoder, Framed},
    net::{TcpListener, TcpStream},
};

mod rules;
mod http_s;
mod sock5;
mod redir;
mod tun;

use crate::outbound::Outbound;
use std::net::ToSocketAddrs;

type MODE = Vec<Box<dyn rules::Rule + Send + Sync>>;

#[derive(Clone, Default, Debug)]
pub struct Config {
    pub address: Option<String>,
    pub modes: Option<String>,
}

#[derive(Debug)]
struct Error {
    v: String,
}

impl Error {
    fn from(v: &str) -> Box<dyn StdError> {
        Box::new(Error { v: From::from(v) })
    }

    fn change_message(&mut self, new_message: &str) {
        self.v = new_message.to_string();
    }
}

impl StdError for Error {
    fn description(&self) -> &str { &self.v }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InboundError: {}", &self.v)
    }
}

pub struct ConnectionMeta {
    pub udp: bool,
    pub host: String,
    pub src_addr: Option<std::net::SocketAddr>,
    pub dst_addr: Option<std::net::SocketAddr>,
}

impl ConnectionMeta {
    pub fn is_host(&self) -> bool {
        !self.host.is_empty()
    }
}

pub struct Engine {
    listener_address: String,
    outbounds: Vec<Box<dyn Outbound>>,
    modes: Arc<HashMap<String, MODE>>,
}

impl Engine {
    #[inline]
    pub fn new(config: &Config) -> Engine {
        let modes = Arc::new(HashMap::new());

        Engine {
            listener_address: config.address.as_ref().unwrap().clone(),
            outbounds: vec![],
            modes,
        }
    }

    pub fn get_modes(&self) -> Vec<&str> {
        self.modes.keys().map(|key| key.as_ref()).collect()
    }

    pub fn update_config(config: &Config) -> Result<(), &'static str> {
        Err("not implement")
    }

    pub fn lookup(&self) {}

    async fn run_rule(stream: &TcpStream, meta: ConnectionMeta)
                      -> Result<(), Box<dyn StdError>> {
        Ok(())
    }

    pub async fn run(&self) -> Result<(), Box<dyn StdError>> {
        let listen_address = &self.listener_address;
        let mut incoming = TcpListener::bind(listen_address).await?.incoming();
        println!("Listening on: {}", listen_address);

        while let Some(Ok(inbound)) = incoming.next().await {
            let modes = self.modes.clone();
            tokio::spawn(async move {
                let mut transport = Framed::new(inbound, http_s::Http);

                while let Some(request) = transport.next().await {
                    let request = match request {
                        Ok(r) => r,
                        Err(e) => {
                            println!("failed to process request {}", e);
                            return;
                        }
                    };

                    let connection_meta = match Engine::build_connection_meta(
                        transport.get_ref(), request).await {
                        Ok(r) => r,
                        Err(e) => {
                            println!("failed to process request {}", e);
                            return;
                        }
                    };

                    let outbound = match Engine::run_rule(
                        transport.get_ref(), connection_meta).await {
                        Ok(r) => r,
                        Err(e) => {
                            println!("failed to process request {}", e);
                            return;
                        }
                    };
                }
            });
        }
        Ok(())
    }

    async fn build_connection_meta(stream: &TcpStream, request: Request<()>)
                                   -> Result<ConnectionMeta, Box<dyn StdError>> {
        let host = match request.uri().host() {
            Some(host) => host,
            None => {
                return Err(Error::from("not have host"));
            }
        };

        let dst_addr = match host.to_socket_addrs() {
            Ok(mut ip) => ip.next(),
            Err(e) => None
        };

        Ok(ConnectionMeta {
            udp: false,
            host: String::from(host),
            dst_addr,
            src_addr: None,
        })
    }

    async fn pipe(inbound: &TcpStream, outbound: &TcpStream) -> Result<(), Box<dyn StdError>> {
        Ok(())
    }

    async fn respond<T>(req: Request<T>) -> Result<Response<String>, Box<dyn StdError>> {
        let mut response = Response::builder();
        let body = match req.uri().path() {
            "/plaintext" => {
                response.header("Content-Type", "text/plain");
                "Hello, World!".to_string()
            }
            "/json" => {
                response.header("Content-Type", "application/json");

                #[derive(Serialize)]
                struct Message {
                    message: &'static str,
                }
                serde_json::to_string(&Message {
                    message: "Hello, World!",
                })?
            }
            _ => {
                response.status(StatusCode::NOT_FOUND);
                String::new()
            }
        };
        let response = response
            .body(body)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

        Ok(response)
    }

    fn delete_hop_by_hop_headers() {}
}

