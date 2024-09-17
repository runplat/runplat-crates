use bytes::{Bytes, BytesMut};
use http_body_util::combinators::BoxBody;
use hyper::{body::Incoming, header, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use reality::{plugin::Bind, BincodeContent, Content, Plugin, Resource, Uuid, Version};
use serde::{Deserialize, Serialize};
use std::{future::Future, path::PathBuf, pin::Pin, sync::OnceLock, task::Poll};
use tokio::{net::TcpStream, select};
use tracing::{debug, error, trace};
use url::Url;

/// Type-alias for the default result type returned by this plugin's plumbing
type Result<T> = std::io::Result<T>;

/// Type-alias for the default http client
type DefaultClient = ClientHelper<Body>;

/// Type-alias for the default request builder
type RequestBuilder = hyper::http::request::Builder;

/// Type-alias for a hyper body
type Body = http_body_util::combinators::BoxBody<Bytes, std::io::Error>;

/// Type-alias for a client helper function for executing a request
type ClientHelper<B> = Box<
    dyn FnOnce(
            hyper::Request<B>,
        )
            -> Pin<Box<dyn Future<Output = Result<Response<Incoming>>> + Send + 'static>>
        + Send,
>;

/// Plugin to execute a request
#[derive(Serialize, Deserialize)]
pub struct Request {
    /// URL to send the request to
    url: Url,
    /// If true, will use http2 when making the request
    #[serde(rename = "http2")]
    use_http2: bool,
    /// File path to use as the body
    file: Option<PathBuf>,
    /// Json string to use as the body
    json: Option<String>,
    /// Response this request received
    #[serde(skip)]
    response: Option<Response<Incoming>>,
}

/// Bytes Body
pub struct BytesBody(Bytes);

/// String body
pub struct StringBody(String);

/// Empty body
pub struct EmptyBody;

impl Plugin for Request {
    fn call(binding: reality::plugin::Bind<Self>) -> reality::Result<reality::plugin::SpawnWork> {
        let plugin = binding.plugin()?;
        if plugin.response.is_some() {
            debug!("Skipping request, response has not been removed");
            binding.skip()
        } else {
            binding.defer(move |mut b, ct| async move {
                let (client, request) = Request::prepare(&b).await?;
                let req_fut = (client)(request);
                let ct_fut = ct.cancelled();
                select! {
                    res = req_fut => {
                        match res {
                            Ok(resp) => {
                                let plugin = b.plugin_mut()?;
                                if plugin.response.is_none() {
                                    plugin.response = Some(resp);
                                    Ok(())
                                } else {
                                    Err(b.plugin_call_error("Response was already set and has not been handled"))
                                }
                            },
                            Err(e) => {
                                Err(b.plugin_call_error(format!("Could not complete sending request {e}")))
                            },
                        }
                    },
                    _ = ct_fut => {
                        Err(reality::Error::PluginCallCancelled)
                    }
                }
            })
        }
    }

    fn version() -> Version {
        env!("CARGO_PKG_VERSION")
            .parse()
            .expect("should be a version because cargo will complain first")
    }
}

impl Request {
    /// Prepares the request to be called
    ///
    /// Returns an error if the binding doesn't match the plugin type, or if the body could not be set on the request
    async fn prepare(
        binding: &Bind<Self>,
    ) -> reality::Result<(DefaultClient, hyper::Request<Body>)> {
        let plugin = binding.plugin()?;
        let url_authority = plugin.url.authority();
        let client = https::<Body>(plugin.use_http2);
        let request = RequestBuilder::new()
            .uri(plugin.url.to_string())
            .header(header::HOST, url_authority)
            .body(plugin.request_body().await);
        Ok((
            client,
            request.map_err(|e| binding.plugin_call_error(e.to_string()))?,
        ))
    }

    /// Returns the request body to use for this request
    async fn request_body(&self) -> Body {
        if let Some(json) = self.json.as_ref() {
            StringBody::from(json.to_string()).into_boxed_body()
        } else if let Some(path) = self.file.as_ref() {
            let body = tokio::fs::read(path).await.unwrap();
            BytesBody::from(BytesMut::from_iter(&body).freeze()).into_boxed_body()
        } else {
            EmptyBody.into_boxed_body()
        }
    }
}

impl Resource for Request {}

impl Content for Request {
    fn state_uuid(&self) -> Uuid {
        BincodeContent::new(self).unwrap().state_uuid()
    }
}

/// Creates a client helper monad that can be used to send an https request
fn https<B>(use_http_2: bool) -> ClientHelper<B>
where
    B: hyper::body::Body + Unpin + Send + Sync + 'static,
    B::Data: Send,
    B::Error: std::error::Error + Send + Sync,
{
    static TLS_CONN: OnceLock<tokio_native_tls::native_tls::TlsConnector> = OnceLock::new();
    static TCP_SOCKET_SEMAPHORE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();

    let tls_conn = TLS_CONN.get_or_init(|| {
        tokio_native_tls::native_tls::TlsConnector::builder()
            .build()
            .expect("should be able to create a new TLS connector")
    });

    let tcp_socket_semaphore = TCP_SOCKET_SEMAPHORE.get_or_init(|| {
        // There are typically ~30000 source ports that can be used
        // They typically are in a TIME_WAIT state for 60 seconds
        // That means there can be about 500 outgoing connections a second at full saturation
        // This semaphore is to safe guard the number of concurrent outgoing connections.
        // For the most part, since the socket is closed after the request is made, it should be more
        // effecient to let mio and the os deal w/ the details of tcp connections.
        tokio::sync::Semaphore::new(400)
    });

    let cx = tokio_native_tls::TlsConnector::from(tls_conn.clone());

    let monad = move |req: hyper::Request<B>| -> Pin<
        Box<dyn Future<Output = Result<Response<Incoming>>> + Send + 'static>,
    > {
        Box::pin(async move {
            let permit = tcp_socket_semaphore
                .acquire()
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::WouldBlock, e.to_string()))?;

            let uri = req.uri();

            if let (Some(authority), Some(host), port) =
                (uri.authority(), uri.host(), uri.port_u16())
            {
                let port = port.unwrap_or(443);

                let addr = format!("{host}:{port}");

                let tcp = TcpStream::connect(addr).await?;

                let stream = cx.connect(authority.as_str(), tcp).await.map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string())
                })?;

                let rt = TokioIo::new(stream);

                if !use_http_2 {
                    let (mut s, conn) = hyper::client::conn::http1::handshake::<_, B>(rt)
                        .await
                        .map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::ConnectionRefused,
                                e.to_string(),
                            )
                        })?;
                    tokio::spawn(async move {
                        let _ = permit;
                        if let Err(err) = conn.await {
                            error!("Connection error {err}");
                        }
                        trace!("Connection is closing");
                    });

                    Ok(s.send_request(req).await.map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e.to_string())
                    })?)
                } else {
                    let (mut s, conn) = hyper::client::conn::http2::handshake::<_, _, B>(
                        TokioExecutor::default(),
                        rt,
                    )
                    .await
                    .map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string())
                    })?;

                    tokio::spawn(async move {
                        let _ = permit;
                        if let Err(err) = conn.await {
                            error!("Connection error {err}");
                        }
                        trace!("Connection is closing");
                    });

                    Ok(s.send_request(req).await.map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e.to_string())
                    })?)
                }
            } else {
                Err::<hyper::Response<Incoming>, std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Uri must have an authority/host",
                ))
            }
        })
    };

    Box::new(monad)
}

/// Trait for converting into a boxed body
trait IntoBoxedBody {
    fn into_boxed_body(self) -> Body;
}

impl<T> IntoBoxedBody for T
where
    T: hyper::body::Body<Data = Bytes, Error = std::io::Error> + Send + Sync + 'static,
{
    fn into_boxed_body(self) -> Body {
        BoxBody::new(self)
    }
}

impl From<Bytes> for BytesBody {
    fn from(value: Bytes) -> Self {
        Self(value)
    }
}

impl hyper::body::Body for BytesBody {
    type Data = Bytes;

    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<std::result::Result<hyper::body::Frame<Self::Data>, Self::Error>>>
    {
        Poll::Ready(Some(Ok(hyper::body::Frame::data(self.0.clone()))))
    }
}

impl From<String> for StringBody {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl hyper::body::Body for StringBody {
    type Data = Bytes;

    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<std::result::Result<hyper::body::Frame<Self::Data>, Self::Error>>>
    {
        Poll::Ready(Some(Ok(hyper::body::Frame::data(Bytes::copy_from_slice(
            self.0.as_bytes(),
        )))))
    }
}

impl hyper::body::Body for EmptyBody {
    type Data = Bytes;

    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<std::result::Result<hyper::body::Frame<Self::Data>, Self::Error>>>
    {
        Poll::Ready(None)
    }
}

impl From<EmptyBody> for Body {
    fn from(value: EmptyBody) -> Self {
        BoxBody::new(value)
    }
}

#[cfg(test)]
mod tests {
    use reality::State;

    use crate::engine::Engine;

    use super::*;

    #[test]
    fn test_request_plugin_name() {
        let name = Request::name();
        assert_eq!("kioto/plugins.request", name.plugin_ref());
    }

    #[tokio::test]
    async fn test_request_plugin_call() {
        let mut state = State::new();

        state
            .load_toml::<Request>(
                r#"
url = "https://jsonplaceholder.typicode.com/posts"
http2 = false
"#,
            )
            .expect("should be able to load request");

        state
            .call("kioto/0.1.0/plugins/request")
            .await
            .expect("should be able to make request");

        let mut plugin = state
            .find_plugin("kioto/0.1.0/plugins/request")
            .expect("should have the plugin")
            .clone();
        let resp = match plugin.borrow_mut::<Request>() {
            Some(r) => r.response.take().expect("should have a response"),
            None => panic!("Should be a request"),
        };
        assert!(resp.status().is_success());
        eprintln!("{:#?}", resp);
        ()
    }

    #[tokio::test]
    async fn test_request_plugin_call_from_engine() {
        let mut state = State::new();
        state
            .load_toml::<Request>(
                r#"
url = "https://jsonplaceholder.typicode.com/posts"
http2 = false
"#,
            )
            .expect("should be able to load request");

        let mut engine = Engine::with(state);
        engine
            .push("kioto/0.1.0/plugins/request")
            .expect("should be able to push an event");
        let event = engine.event(0).expect("should have this event");

        let (event, _) = event.fork();
        event.start().await.expect("should work");

        let resp = engine
            .state()
            .find_plugin("kioto/0.1.0/plugins/request")
            .map(|i| i.clone())
            .and_then(|mut i| i.borrow_mut::<Request>().and_then(|r| r.response.take()))
            .expect("should return resp");
        assert!(resp.status().is_success());
        eprintln!("{:#?}", resp);
        ()
    }
}
