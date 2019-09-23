use crate::outbound::Direct;
use crate::outbound::Outbound;
use crate::rules;
use crate::rules::{build_modes, lookup};
use actix::prelude::*;
use actix_net::server::Server;
use actix_service::{NewService, Service};
use actix_web::client::Client;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::future;
use futures::prelude::*;
use log::{debug, error, info};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

fn build_connection_meta<T>(
    request: &HttpRequest,
) -> Result<rules::ConnectionMeta, Box<dyn std::error::Error>> {
    let host = match request.uri().host() {
        Some(host) => host.to_owned(),
        None => {
            return Err(From::from("not have host"));
        }
    };

    let dst_addr = match host.to_socket_addrs() {
        Ok(mut addrs) => addrs.next(),
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    Ok(rules::ConnectionMeta {
        udp: false,
        host,
        dst_addr,
        src_addr: request.peer_addr(),
    })
}

pub fn setup_http_inbounds() -> std::io::Result<()> {
    HttpServer::new(move || App::new().wrap(middleware::Logger::default()))
        .bind("127.0.0.1:59090")?
        .start();
    Ok(())
}

//struct HttpServer {}
//
//#[derive(Message)]
//struct TcpConnect(pub TcpStream, pub SocketAddr);
//
//impl Handler<TcpConnect> for HttpServer {
//    type Result = ();
//
//    fn handle(&mut self, msg: TcpConnect, _: &mut Context<Self>) {
//        Server::new().
//    }
//}
