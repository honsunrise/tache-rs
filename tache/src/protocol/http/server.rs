use log::{debug, error, info};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::{fmt, io, net};

use actix::prelude::*;
use actix_http::Request;
use actix_server::{Server, ServerBuilder};
use actix_server_config::{Io as ServerIo, IoStream, Protocol, ServerConfig};
use actix_service::{IntoNewService, NewService, Service};
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::future;
use futures::prelude::*;
use net2::TcpBuilder;

use crate::protocol::http::service::HttpProxyService;

struct Error {}

struct Config {
    client_timeout: u64,
    client_shutdown: u64,
}

pub struct HttpProxyServer<F, I, S>
where
    F: Fn() -> I + Send + Clone + 'static,
    I: IntoNewService<S>,
    S: NewService<Config = ServerConfig, Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::InitError: fmt::Debug,
    S::Service: 'static,
{
    pub(super) factory: F,
    pub(super) host: Option<String>,
    config: Arc<Mutex<Config>>,
    backlog: i32,
    sockets: Vec<net::SocketAddr>,
    builder: ServerBuilder,
    _t: PhantomData<(S)>,
}

impl<F, I, S> HttpProxyServer<F, I, S>
where
    F: Fn() -> I + Send + Clone + 'static,
    I: IntoNewService<S>,
    S: NewService<Config = ServerConfig, Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::InitError: fmt::Debug,
    S::Service: 'static,
{
    /// Create new http server with application factory
    pub fn new(factory: F) -> Self {
        HttpProxyServer {
            factory,
            host: None,
            config: Arc::new(Mutex::new(Config {
                client_timeout: 5000,
                client_shutdown: 5000,
            })),
            backlog: 1024,
            sockets: Vec::new(),
            builder: ServerBuilder::default(),
            _t: PhantomData,
        }
    }

    pub fn workers(mut self, num: usize) -> Self {
        self.builder = self.builder.workers(num);
        self
    }

    pub fn backlog(mut self, backlog: i32) -> Self {
        self.backlog = backlog;
        self.builder = self.builder.backlog(backlog);
        self
    }

    pub fn maxconn(mut self, num: usize) -> Self {
        self.builder = self.builder.maxconn(num);
        self
    }

    pub fn maxconnrate(mut self, num: usize) -> Self {
        self.builder = self.builder.maxconnrate(num);
        self
    }

    pub fn client_timeout(self, val: u64) -> Self {
        self.config.lock().unwrap().client_timeout = val;
        self
    }

    pub fn client_shutdown(self, val: u64) -> Self {
        self.config.lock().unwrap().client_shutdown = val;
        self
    }

    pub fn server_hostname<T: AsRef<str>>(mut self, val: T) -> Self {
        self.host = Some(val.as_ref().to_owned());
        self
    }

    pub fn system_exit(mut self) -> Self {
        self.builder = self.builder.system_exit();
        self
    }

    pub fn disable_signals(mut self) -> Self {
        self.builder = self.builder.disable_signals();
        self
    }

    pub fn shutdown_timeout(mut self, sec: u64) -> Self {
        self.builder = self.builder.shutdown_timeout(sec);
        self
    }

    pub fn listen(mut self, lst: net::TcpListener) -> io::Result<Self> {
        let cfg = self.config.clone();
        let factory = self.factory.clone();
        let addr = lst.local_addr().unwrap();
        self.sockets.push(addr);

        self.builder = self.builder.listen(
            format!("actix-http-proxy-server-{}", addr),
            lst,
            move || {
                let c = cfg.lock().unwrap();
                HttpProxyService::new(factory())
            },
        )?;
        Ok(self)
    }

    pub fn bind<A: net::ToSocketAddrs>(mut self, addr: A) -> io::Result<Self> {
        let sockets = self.bind2(addr)?;

        for lst in sockets {
            self = self.listen(lst)?;
        }

        Ok(self)
    }

    fn bind2<A: net::ToSocketAddrs>(&self, addr: A) -> io::Result<Vec<net::TcpListener>> {
        let mut err = None;
        let mut succ = false;
        let mut sockets = Vec::new();
        for addr in addr.to_socket_addrs()? {
            match create_tcp_listener(addr, self.backlog) {
                Ok(lst) => {
                    succ = true;
                    sockets.push(lst);
                }
                Err(e) => err = Some(e),
            }
        }

        if !succ {
            if let Some(e) = err.take() {
                Err(e)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Can not bind to address.",
                ))
            }
        } else {
            Ok(sockets)
        }
    }

    pub fn start(self) -> Server {
        self.builder.start()
    }

    pub fn run(self) -> io::Result<()> {
        let sys = System::new("http-server");
        self.start();
        sys.run()
    }
}

fn create_tcp_listener(addr: net::SocketAddr, backlog: i32) -> io::Result<net::TcpListener> {
    let builder = match addr {
        net::SocketAddr::V4(_) => TcpBuilder::new_v4()?,
        net::SocketAddr::V6(_) => TcpBuilder::new_v6()?,
    };
    builder.reuse_address(true)?;
    builder.bind(addr)?;
    Ok(builder.listen(backlog)?)
}
