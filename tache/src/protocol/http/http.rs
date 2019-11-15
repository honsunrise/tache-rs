use std::future::Future;

use std::pin::Pin;

use std::io;
use std::task::{Context, Poll};

use futures::ready;
use futures::StreamExt;
use http::{header::HeaderValue, Request, Response, Uri};

use async_std::io::BufRead;

/// Future for the [`read_until`](crate::io::AsyncBufReadExt::read_until) method.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadHttpRequest<'a, R: ?Sized + Unpin> {
    reader: &'a mut R,
}

impl<R: ?Sized + Unpin> Unpin for ReadHttpRequest<'_, R> {}

pub fn read_http<R>(reader: &mut R) -> ReadHttpRequest<R>
where
    R: BufRead + ?Sized + Unpin,
{
    ReadHttpRequest { reader }
}

impl<R: BufRead + ?Sized + Unpin> Future for ReadHttpRequest<'_, R> {
    type Output = io::Result<Request<()>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { reader } = &mut *self;
        let mut reader = Pin::new(reader);
        let mut headers: usize = 16;
        loop {
            let available = ready!(reader.as_mut().poll_fill_buf(cx))?;
            let mut parsed_headers = vec![httparse::EMPTY_HEADER; headers];
            let mut r = httparse::Request::new(&mut parsed_headers[..]);
            let status = match r.parse(available) {
                Err(e) => {
                    if e == httparse::Error::TooManyHeaders {
                        headers += 10;
                        continue;
                    }
                    let msg = format!("failed to parse http request: {:?}", e);
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, msg)));
                }
                Ok(status) => status,
            };

            let amt = match status {
                httparse::Status::Complete(amt) => amt,
                httparse::Status::Partial => {
                    continue;
                }
            };

            if r.version.unwrap() != 1 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    "only HTTP/1.1 accepted",
                )));
            }

            let mut ret = Request::builder();
            ret.version(http::Version::HTTP_11);
            ret.method(r.method.unwrap());
            for (_i, header) in r.headers.iter().enumerate() {
                let k = header.name.as_bytes();
                let v = header.value;
                ret.header(k, v);
            }
            let uri = Uri::builder()
                .scheme("http")
                .authority(ret.headers_ref().unwrap().get("host").unwrap().as_bytes())
                .path_and_query(r.path.unwrap())
                .build()
                .unwrap();
            ret.uri(uri);

            let result = match ret.body(()) {
                Err(e) => {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                }
                Ok(result) => result,
            };
            reader.as_mut().consume(amt + 1);
            return Poll::Ready(Ok(result));
        }
    }
}

mod date {
    use std::cell::RefCell;
    use std::fmt::{self, Write};
    use std::str;

    use time::{self, Duration};

    pub struct Now(());

    /// Returns a struct, which when formatted, renders an appropriate `Date`
    /// header value.
    pub fn now() -> Now {
        Now(())
    }

    // Gee Alex, doesn't this seem like premature optimization. Well you see
    // there Billy, you're absolutely correct! If your server is *bottlenecked*
    // on rendering the `Date` header, well then boy do I have news for you, you
    // don't need this optimization.
    //
    // In all seriousness, though, a simple "hello world" benchmark which just
    // sends back literally "hello world" with standard headers actually is
    // bottlenecked on rendering a date into a byte buffer. Since it was at the
    // top of a profile, and this was done for some competitive benchmarks, this
    // module was written.
    //
    // Just to be clear, though, I was not intending on doing this because it
    // really does seem kinda absurd, but it was done by someone else [1], so I
    // blame them!  :)
    //
    // [1]: https://github.com/rapidoid/rapidoid/blob/f1c55c0555007e986b5d069fe1086e6d09933f7b/rapidoid-commons/src/main/java/org/rapidoid/commons/Dates.java#L48-L66

    struct LastRenderedNow {
        bytes: [u8; 128],
        amt: usize,
        next_update: time::Timespec,
    }

    thread_local!(static LAST: RefCell<LastRenderedNow> = RefCell::new(LastRenderedNow {
        bytes: [0; 128],
        amt: 0,
        next_update: time::Timespec::new(0, 0),
    }));

    impl fmt::Display for Now {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            LAST.with(|cache| {
                let mut cache = cache.borrow_mut();
                let now = time::get_time();
                if now >= cache.next_update {
                    cache.update(now);
                }
                f.write_str(cache.buffer())
            })
        }
    }

    impl LastRenderedNow {
        fn buffer(&self) -> &str {
            str::from_utf8(&self.bytes[..self.amt]).unwrap()
        }

        fn update(&mut self, now: time::Timespec) {
            self.amt = 0;
            write!(LocalBuffer(self), "{}", time::at(now).rfc822()).unwrap();
            self.next_update = now + Duration::seconds(1);
            self.next_update.nsec = 0;
        }
    }

    struct LocalBuffer<'a>(&'a mut LastRenderedNow);

    impl fmt::Write for LocalBuffer<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let start = self.0.amt;
            let end = start + s.len();
            self.0.bytes[start..end].copy_from_slice(s.as_bytes());
            self.0.amt += s.len();
            Ok(())
        }
    }
}

fn delete_hop_by_hop_headers() {}
