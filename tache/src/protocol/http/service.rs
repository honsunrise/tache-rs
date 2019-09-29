use log::{debug, error, info};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{fmt, io, net};

use crate::protocol::http::service::TimerKind::SlowRequest;
use actix::prelude::*;
use actix_http::body::{Body, ResponseBody};
use actix_http::error::ParseError;
use actix_http::h1::{Codec, Message, MessageType};
use actix_http::{KeepAlive, Request, ServiceConfig};
use actix_server::{Server, ServerBuilder};
use actix_server_config::{Io as ServerIo, IoStream, Protocol, ServerConfig as SrvConfig};
use actix_service::{IntoNewService, NewService, Service};
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use bitflags::bitflags;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::future;
use futures::{try_ready, Async, Future, IntoFuture, Poll};
use net2::TcpBuilder;
use tokio::codec::Decoder;
use tokio::timer::Delay;

const LW_BUFFER_SIZE: usize = 4096;
const HW_BUFFER_SIZE: usize = 32_768;

bitflags! {
    pub struct Flags: u8 {
        const STARTED            = 0b0000_0001;
        const KEEPALIVE          = 0b0000_0010;
        const POLLED             = 0b0000_0100;
        const SHUTDOWN           = 0b0000_1000;
        const READHALF_CLOSED    = 0b0001_0000;
        const WRITEHALF_CLOSED   = 0b0010_0000;
        const UPGRADE            = 0b0100_0000;
    }
}

pub struct Error {}

pub struct HttpProxyService<T, P, S> {
    srv: S,
    cfg: ServiceConfig,
    _t: PhantomData<(T, P)>,
}

impl<T, P, S> HttpProxyService<T, P, S>
where
    S: NewService<Config = SrvConfig, Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::InitError: fmt::Debug,
    <S::Service as Service>::Future: 'static,
{
    pub fn new<F: IntoNewService<S>>(service: F) -> Self {
        let cfg = ServiceConfig::new(KeepAlive::Timeout(5), 5000, 0);

        HttpProxyService {
            cfg,
            srv: service.into_new_service(),
            _t: PhantomData,
        }
    }
}

impl<T, P, S> NewService for HttpProxyService<T, P, S>
where
    T: IoStream,
    S: NewService<Config = SrvConfig, Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::InitError: fmt::Debug,
    <S::Service as Service>::Future: 'static,
{
    type Request = ServerIo<T, P>;
    type Response = ();
    type Error = Error;
    type Config = SrvConfig;
    type Service = HttpProxyServiceHandler<T, P, S::Service>;
    type InitError = ();
    type Future = HttpProxyServiceResponse<T, P, S>;

    fn new_service(&self, cfg: &Self::Config) -> Self::Future {
        HttpProxyServiceResponse {
            fut: self.srv.new_service(cfg).into_future(),
            cfg: Some(self.cfg.clone()),
            _t: PhantomData,
        }
    }
}

struct HttpProxyServiceResponse<T, P, S: NewService> {
    fut: S::Future,
    cfg: Option<ServiceConfig>,
    _t: PhantomData<(T, P)>,
}

impl<T, P, S> Future for HttpProxyServiceResponse<T, P, S>
where
    T: IoStream,
    S: NewService<Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::InitError: fmt::Debug,
    <S::Service as Service>::Future: 'static,
{
    type Item = HttpProxyServiceHandler<T, P, S::Service>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let service = try_ready!(self
            .fut
            .poll()
            .map_err(|e| log::error!("Init http proxy service error: {:?}", e)));

        Ok(Async::Ready(HttpProxyServiceHandler::new(
            self.cfg.take().unwrap(),
            service,
        )))
    }
}

struct HttpProxyServiceHandler<T, P, S> {
    srv: S,
    cfg: ServiceConfig,
    _t: PhantomData<(T, P)>,
}

impl<T, P, S> HttpProxyServiceHandler<T, P, S>
where
    T: IoStream,
    S: Service<Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::Future: 'static,
{
    fn new(cfg: ServiceConfig, srv: S) -> Self {
        HttpProxyServiceHandler {
            cfg,
            srv,
            _t: PhantomData,
        }
    }
}

impl<T, P, S> Service for HttpProxyServiceHandler<T, P, S>
where
    T: IoStream,
    S: Service<Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::Future: 'static,
{
    type Request = ServerIo<T, P>;
    type Response = ();
    type Error = Error;
    type Future = HttpServiceHandlerResponse<T, S>;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        let ready = self
            .srv
            .poll_ready()
            .map_err(|e| {
                let e = e.into();
                log::error!("Http service readiness error: {:?}", e);
                DispatchError::Service(e)
            })?
            .is_ready();

        if ready {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        let (mut io, _, proto) = req.into_parts();
        match proto {
            Protocol::Http10 | Protocol::Http11 => {
                HttpServiceHandlerResponse::new(self.cfg.clone(), io, self.srv)
            }
            _ => panic!(),
        }
    }
}

enum TimerKind {
    SlowRequest(Delay),
    HttpKeepalive(Delay),
    Shutdown(Delay),
}

struct HttpServiceHandlerResponse<T, S> {
    cfg: ServiceConfig,
    io: T,
    read_buf: BytesMut,
    flags: Flags,
    shutdown: bool,
    timer: Option<TimerKind>,
    codec: Codec,
    write_buf: BytesMut,
    _t: PhantomData<(T, S)>,
}

impl<T, S> HttpServiceHandlerResponse<T, S>
where
    T: IoStream,
    S: Service<Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::Future: 'static,
{
    fn new(cfg: ServiceConfig, io: T, srv: S) -> Self {
        // timer
        if let Some(delay) = cfg.client_timer() {
            Some(SlowRequest(delay))
        } else if let Some(delay) = cfg.keep_alive_timer() {
            Some(SlowRequest(delay))
        } else {
            None
        };

        HttpServiceHandlerResponse {
            cfg,
            io,
            read_buf: BytesMut::with_capacity(4096),
            flags: Flags::empty(),
            shutdown: false,
            timer,
            codec: Codec::new(config.clone()),
            write_buf: BytesMut::with_capacity(4096),
            _t: PhantomData,
        }
    }

    fn handle_request(&mut self, req: Request) -> Result<State<S, B, X>, Error> {
        // Call service
        let mut task = self.srv.call(req);
        match task.poll() {
            Ok(Async::Ready(res)) => {
                let (res, body) = res.into().replace_body(());
                self.send_response(res, body)
            }
            Ok(Async::NotReady) => Ok(State::ServiceCall(task)),
            Err(e) => {
                let res: Response = e.into().into();
                let (res, body) = res.replace_body(());
                self.send_response(res, body.into_body())
            }
        }
    }

    fn poll_flush(&mut self) -> Poll<(), Error> {}
}

impl<T, S> Future for HttpServiceHandlerResponse<T, S>
where
    T: IoStream,
    S: Service<Request = Request, Response = ()>,
    S::Error: Into<Error>,
    S::Future: 'static,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if !self.shutdown {
            // read socket into a buf
            let should_disconnect = read_available(&mut io, &mut self.read_buf)?;

            if should_disconnect {
                self.flags.insert(Flags::READHALF_CLOSED);
            };

            // process request
            loop {
                match self.codec.decode(&mut self.read_buf) {
                    Ok(Some(mut req)) => {
                        let pl = self.codec.message_type();
                        req.head_mut().peer_addr = self.io.peer_addr();
                        self.handle_request(req)?;

                        // update timer for keepalive
                        if let Some(TimerKind::HttpKeepalive(timer)) = self.timer.as_mut() {
                            if let Some(deadline) = self.codec.config().keep_alive_expire() {
                                timer.reset(deadline);
                            }
                        } else if let Some(timer) = self.cfg.keep_alive_timer() {
                            self.timer = Some(TimerKind::HttpKeepalive(timer))
                        } else {
                            self.timer = None;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        //TODO: send bad request
                        self.flags.insert(Flags::READHALF_CLOSED);
                        break;
                    }
                }
            }
        // process response
        } else {
            try_ready!(self.poll_flush());
            return match self.io.shutdown()? {
                Async::Ready(_) => Ok(Async::Ready(())),
                Async::NotReady => Ok(Async::NotReady),
            };
        }

        // process timer
        match self.timer.as_mut() {
            Some(TimerKind::SlowRequest(timer)) => match timer.poll()? {
                Async::Ready(_) => {
                    trace!("Slow request timeout");
                    let _ = self.send_response(
                        Response::RequestTimeout().finish().drop_body(),
                        ResponseBody::Other(Body::Empty),
                    );
                    self.shutdown = true;
                }
                Async::NotReady => (),
            },
            Some(TimerKind::HttpKeepalive(timer)) => match timer.poll()? {
                Async::Ready(_) => {
                    trace!("Keep-alive timeout, close connection");
                    self.shutdown = true;

                    // start shutdown timer if exist
                    if let Some(deadline) = self.codec.config().client_disconnect_timer() {
                        self.timer = Some(TimerKind::Shutdown(Delay::new(deadline)));
                    }
                }
                Async::NotReady => (),
            },
            Some(TimerKind::Shutdown(timer)) => match timer.poll()? {
                Async::Ready(_) => {
                    return Err(DispatchError::DisconnectTimeout);
                }
                Async::NotReady => (),
            },
            None => {
                // noting to do
            }
        }

        Ok(Async::NotReady)
    }
}

fn read_available<T>(io: &mut T, buf: &mut BytesMut) -> Result<bool, io::Error>
where
    T: io::Read,
{
    let mut read_some = false;
    loop {
        if buf.remaining_mut() < LW_BUFFER_SIZE {
            buf.reserve(HW_BUFFER_SIZE);
        }

        let read = unsafe { io.read(buf.bytes_mut()) };
        match read {
            Ok(n) => {
                if n == 0 {
                    return Ok(true);
                } else {
                    read_some = true;
                    unsafe {
                        buf.advance_mut(n);
                    }
                }
            }
            Err(e) => {
                return if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(false)
                } else if e.kind() == io::ErrorKind::ConnectionReset && read_some {
                    Ok(true)
                } else {
                    Err(e)
                };
            }
        }
    }
}
