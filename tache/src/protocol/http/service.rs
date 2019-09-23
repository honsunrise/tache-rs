use log::{debug, error, info};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{fmt, io, net};

use actix::prelude::*;
use actix_http::error::ParseError;
use actix_http::h1::Codec;
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

bitflags! {
    pub struct Flags: u8 {
        const STARTED            = 0b0000_0001;
        const KEEPALIVE          = 0b0000_0010;
        const POLLED             = 0b0000_0100;
        const SHUTDOWN           = 0b0000_1000;
        const READ_DISCONNECT    = 0b0001_0000;
        const WRITE_DISCONNECT   = 0b0010_0000;
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

struct HttpServiceHandlerResponse<T, S> {
    cfg: ServiceConfig,
    io: T,
    read_buf: BytesMut,
    flags: Flags,
    ka_expire: Instant,
    ka_timer: Option<Delay>,
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
        let keepalive = config.keep_alive_enabled();
        let flags = if keepalive {
            Flags::KEEPALIVE
        } else {
            Flags::empty()
        };

        // keep-alive timer
        let (ka_expire, ka_timer) = if let Some(delay) = timeout {
            (delay.deadline(), Some(delay))
        } else if let Some(delay) = config.keep_alive_timer() {
            (delay.deadline(), Some(delay))
        } else {
            (config.now(), None)
        };

        HttpServiceHandlerResponse {
            cfg,
            io,
            read_buf: BytesMut::with_capacity(4096),
            flags,
            ka_expire,
            ka_timer,
            codec: Codec::new(config.clone()),
            write_buf: BytesMut::with_capacity(4096),
            _t: PhantomData,
        }
    }

    fn can_read(&self) -> bool {
        !self.flags.intersects(Flags::READ_DISCONNECT)
    }

    fn client_disconnected(&mut self) {
        self.flags
            .insert(Flags::READ_DISCONNECT | Flags::WRITE_DISCONNECT);
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

    /// Process one incoming requests
    pub(self) fn poll_request(&mut self) -> Result<bool, Error> {
        // limit a mount of non processed requests
        if self.messages.len() >= MAX_PIPELINED_MESSAGES || !self.can_read() {
            return Ok(false);
        }

        let mut updated = false;
        loop {
            match self.codec.decode(&mut self.read_buf) {
                Ok(Some(msg)) => {
                    updated = true;
                    self.flags.insert(Flags::STARTED);

                    match msg {
                        Message::Item(mut req) => {
                            let pl = self.codec.message_type();
                            req.head_mut().peer_addr = self.peer_addr;

                            // set on_connect data
                            if let Some(ref on_connect) = self.on_connect {
                                on_connect.set(&mut req.extensions_mut());
                            }

                            if pl == MessageType::Stream && self.upgrade.is_some() {
                                self.messages.push_back(DispatcherMessage::Upgrade(req));
                                break;
                            }
                            if pl == MessageType::Payload || pl == MessageType::Stream {
                                let (ps, pl) = Payload::create(false);
                                let (req1, _) = req.replace_payload(crate::Payload::H1(pl));
                                req = req1;
                                self.payload = Some(ps);
                            }

                            // handle request early
                            if self.state.is_empty() {
                                self.state = self.handle_request(req)?;
                            } else {
                                self.messages.push_back(DispatcherMessage::Item(req));
                            }
                        }
                        Message::Chunk(Some(chunk)) => {
                            if let Some(ref mut payload) = self.payload {
                                payload.feed_data(chunk);
                            } else {
                                error!("Internal server error: unexpected payload chunk");
                                self.flags.insert(Flags::READ_DISCONNECT);
                                self.messages.push_back(DispatcherMessage::Error(
                                    Response::InternalServerError().finish().drop_body(),
                                ));
                                self.error = Some(DispatchError::InternalError);
                                break;
                            }
                        }
                        Message::Chunk(None) => {
                            if let Some(mut payload) = self.payload.take() {
                                payload.feed_eof();
                            } else {
                                error!("Internal server error: unexpected eof");
                                self.flags.insert(Flags::READ_DISCONNECT);
                                self.messages.push_back(DispatcherMessage::Error(
                                    Response::InternalServerError().finish().drop_body(),
                                ));
                                self.error = Some(DispatchError::InternalError);
                                break;
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(ParseError::Io(e)) => {
                    self.client_disconnected();
                    self.error = Some(DispatchError::Io(e));
                    break;
                }
                Err(e) => {
                    if let Some(mut payload) = self.payload.take() {
                        payload.set_error(PayloadError::EncodingCorrupted);
                    }

                    // Malformed requests should be responded with 400
                    self.messages.push_back(DispatcherMessage::Error(
                        Response::BadRequest().finish().drop_body(),
                    ));
                    self.flags.insert(Flags::READ_DISCONNECT);
                    self.error = Some(e.into());
                    break;
                }
            }
        }

        if updated && self.ka_timer.is_some() {
            if let Some(expire) = self.codec.config().keep_alive_expire() {
                self.ka_expire = expire;
            }
        }
        Ok(updated)
    }

    /// keep-alive timer
    fn poll_keepalive(&mut self) -> Result<(), DispatchError> {
        if self.ka_timer.is_none() {
            // shutdown timeout
            if self.flags.contains(Flags::SHUTDOWN) {
                if let Some(interval) = self.codec.config().client_disconnect_timer() {
                    self.ka_timer = Some(Delay::new(interval));
                } else {
                    self.flags.insert(Flags::READ_DISCONNECT);
                    if let Some(mut payload) = self.payload.take() {
                        payload.set_error(PayloadError::Incomplete(None));
                    }
                    return Ok(());
                }
            } else {
                return Ok(());
            }
        }

        match self.ka_timer.as_mut().unwrap().poll().map_err(|e| {
            error!("Timer error {:?}", e);
            DispatchError::Unknown
        })? {
            Async::Ready(_) => {
                // if we get timeout during shutdown, drop connection
                if self.flags.contains(Flags::SHUTDOWN) {
                    return Err(DispatchError::DisconnectTimeout);
                } else if self.ka_timer.as_mut().unwrap().deadline() >= self.ka_expire {
                    // check for any outstanding tasks
                    if self.state.is_empty() && self.write_buf.is_empty() {
                        if self.flags.contains(Flags::STARTED) {
                            trace!("Keep-alive timeout, close connection");
                            self.flags.insert(Flags::SHUTDOWN);

                            // start shutdown timer
                            if let Some(deadline) = self.codec.config().client_disconnect_timer() {
                                if let Some(timer) = self.ka_timer.as_mut() {
                                    timer.reset(deadline);
                                    let _ = timer.poll();
                                }
                            } else {
                                // no shutdown timeout, drop socket
                                self.flags.insert(Flags::WRITE_DISCONNECT);
                                return Ok(());
                            }
                        } else {
                            // timeout on first request (slow request) return 408
                            if !self.flags.contains(Flags::STARTED) {
                                trace!("Slow request timeout");
                                let _ = self.send_response(
                                    Response::RequestTimeout().finish().drop_body(),
                                    ResponseBody::Other(Body::Empty),
                                );
                            } else {
                                trace!("Keep-alive connection timeout");
                            }
                            self.flags.insert(Flags::STARTED | Flags::SHUTDOWN);
                            self.state = State::None;
                        }
                    } else if let Some(deadline) = self.codec.config().keep_alive_expire() {
                        if let Some(timer) = self.ka_timer.as_mut() {
                            timer.reset(deadline);
                            let _ = timer.poll();
                        }
                    }
                } else if let Some(timer) = self.ka_timer.as_mut() {
                    timer.reset(self.ka_expire);
                    let _ = timer.poll();
                }
            }
            Async::NotReady => (),
        }

        Ok(())
    }
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
        self.poll_keepalive()?;

        if self.flags.contains(Flags::SHUTDOWN) {
            if self.flags.contains(Flags::WRITE_DISCONNECT) {
                Ok(Async::Ready(()))
            } else {
                // flush buffer
                self.poll_flush()?;
                if !self.write_buf.is_empty() {
                    Ok(Async::NotReady)
                } else {
                    match self.io.shutdown()? {
                        Async::Ready(_) => Ok(Async::Ready(())),
                        Async::NotReady => Ok(Async::NotReady),
                    }
                }
            }
        } else {
            // read socket into a buf
            let should_disconnect = if !self.flags.contains(Flags::READ_DISCONNECT) {
                read_available(&mut io, &mut self.read_buf)?
            } else {
                None
            };

            self.poll_request()?;
            if let Some(true) = should_disconnect {
                self.flags.insert(Flags::READ_DISCONNECT);
                if let Some(mut payload) = self.payload.take() {
                    payload.feed_eof();
                }
            };

            loop {
                if self.write_buf.remaining_mut() < LW_BUFFER_SIZE {
                    self.write_buf.reserve(HW_BUFFER_SIZE);
                }
                let result = self.poll_response()?;
                let drain = result == PollResponse::DrainWriteBuf;

                // we didnt get WouldBlock from write operation,
                // so data get written to kernel completely (OSX)
                // and we have to write again otherwise response can get stuck
                if self.poll_flush()? || !drain {
                    break;
                }
            }

            // client is gone
            if self.flags.contains(Flags::WRITE_DISCONNECT) {
                return Ok(Async::Ready(()));
            }

            let is_empty = self.state.is_empty();

            // read half is closed and we do not processing any responses
            if self.flags.contains(Flags::READ_DISCONNECT) && is_empty {
                self.flags.insert(Flags::SHUTDOWN);
            }

            // keep-alive and stream errors
            if is_empty && self.write_buf.is_empty() {
                if let Some(err) = self.error.take() {
                    Err(err)
                }
                // disconnect if keep-alive is not enabled
                else if self.flags.contains(Flags::STARTED)
                    && !self.flags.intersects(Flags::KEEPALIVE)
                {
                    self.flags.insert(Flags::SHUTDOWN);
                    self.poll()
                }
                // disconnect if shutdown
                else if self.flags.contains(Flags::SHUTDOWN) {
                    self.poll()
                } else {
                    Ok(Async::NotReady)
                }
            } else {
                Ok(Async::NotReady)
            }
        }
    }
}

fn read_available<T>(io: &mut T, buf: &mut BytesMut) -> Result<Option<bool>, io::Error>
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
                    return Ok(Some(true));
                } else {
                    read_some = true;
                    unsafe {
                        buf.advance_mut(n);
                    }
                }
            }
            Err(e) => {
                return if e.kind() == io::ErrorKind::WouldBlock {
                    if read_some {
                        Ok(Some(false))
                    } else {
                        Ok(None)
                    }
                } else if e.kind() == io::ErrorKind::ConnectionReset && read_some {
                    Ok(Some(true))
                } else {
                    Err(e)
                };
            }
        }
    }
}
