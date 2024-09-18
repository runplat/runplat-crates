use bytes::{Bytes, BytesMut};
use clap::Args;
use http_body_util::combinators::BoxBody;
use hyper::{
    body::Incoming, header, Response
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use reality::{plugin::Bind, BincodeContent, Content, Plugin, Resource, Uuid, Version};
use serde::{Deserialize, Serialize};
use std::{future::Future, path::PathBuf, pin::Pin, sync::OnceLock, task::Poll};
use tokio::{net::TcpStream, select};
use tracing::{debug, error, trace, warn};
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

/// Arguments for an HTTP request
#[derive(Args, Serialize)]
pub struct RequestArgs {
    /// Headers to pass with the request
    ///
    /// Can be passed multiple times,
    ///
    /// # Example
    /// -H 'accept=application/json'
    /// --header 'accept=application/json'
    #[clap(short = 'H', long)]
    header: Vec<String>,
    /// Sets the method to a POST request
    #[clap(long, action)]
    post: bool,
    /// Sets the method to a PUT request
    #[clap(long, action)]
    put: bool,
    /// Sets the method to a PATCH request
    #[clap(long, action)]
    patch: bool,
    /// Sets the method to a DELETE request
    #[clap(long, action)]
    delete: bool,
    /// JSON body to set with this request
    #[clap(long)]
    json: Option<String>,
    /// File path to read and include with the request
    #[clap(short, long)]
    file: Option<PathBuf>,
    /// Will use http2
    #[clap(long = "http2")]
    use_http2: bool,
    /// Url to send the request to
    url: Url,
    /// Built request when this plugin is loaded
    #[clap(skip)]
    request: Option<Request>,
}

impl Plugin for RequestArgs {
    fn call(bind: Bind<Self>) -> reality::Result<reality::plugin::SpawnWork> {
        let plugin = bind.plugin()?;
        if plugin.request.is_none() {
            bind.skip()
        } else {
            bind.defer(|mut i, ct| async move {
                let binding = i.clone();
                let req = i.plugin_mut()?;
                if let Some(req) = req.request.as_mut() {
                    let request = req
                        .create_request()
                        .await
                        .map_err(|e| binding.plugin_call_error(e.to_string()))?;

                    let req_fut = req.client()(request);
                    let ct_fut = ct.cancelled();

                    select! {
                        resp = req_fut => {
                            match resp {
                                Ok(resp) => {
                                    if req.response.is_some() {
                                        Err(binding.plugin_call_error("Response was already set and has not been handled"))
                                    } else {
                                        req.response = Some(resp);
                                        Ok(())
                                    }
                                },
                                Err(err) => {
                                    Err(binding.plugin_call_error(err.to_string()))
                                },
                            }
                        },
                        _ = ct_fut => {
                            Err(binding.plugin_call_cancelled())
                        }
                    }
                } else {
                    Err(binding.plugin_call_error("Request was not loaded"))
                }
            })
        }
    }

    fn version() -> Version {
        env!("CARGO_PKG_VERSION")
            .parse()
            .expect("should be a version because cargo will complain first")
    }

    fn load(mut put: reality::runir::store::Put<'_, Self>) -> reality::runir::store::Put<'_, Self> {
        let request = {
            let args = put.resource();
            let mut request = Request::new(args.url.clone());
            if args.delete {
                request.method = Some("DELETE".to_string());
            } else if args.patch {
                request.method = Some("PATCH".to_string());
            } else if args.post {
                request.method = Some("POST".to_string());
            } else if args.put {
                request.method = Some("PUT".to_string());
            }
            request.use_http2 = args.use_http2;
            request.headers = args
                .header
                .iter()
                .map(|h| h.trim().trim_matches(['\'', '"']).to_string())
                .collect();
            request.file = args.file.clone();
            request.json = args.json.clone();
            request
        };
        put.resource_mut().request = Some(request);
        put
    }
}

impl RequestArgs {
    /// Takes the response from the request args
    #[inline]
    pub fn take_response(&mut self) -> Option<Response<Incoming>> {
        self.request.as_mut().and_then(|r| r.response.take())
    }

    /// Returns a mutable reference to the inner request
    #[inline]
    pub fn take_request(&mut self) -> Option<Request> {
        self.request.take()
    }

    /// Returns a reference to the request
    #[inline]
    pub fn request(&self) -> Option<&Request> {
        self.request.as_ref()
    }

    /// Returns a mutable reference to the inner request
    #[inline]
    pub fn request_mut(&mut self) -> Option<&mut Request> {
        self.request.as_mut()
    }
}

impl Resource for RequestArgs {}
impl Content for RequestArgs {
    fn state_uuid(&self) -> Uuid {
        BincodeContent::new(self).unwrap().state_uuid()
    }
}

/// Plugin to execute a request
#[derive(Serialize, Deserialize)]
pub struct Request {
    /// URL to send the request to
    url: Url,
    /// If true, will use http2 when making the request
    #[serde(rename = "http2", default)]
    use_http2: bool,
    /// File path to use as the body
    file: Option<PathBuf>,
    /// Json string to use as the body
    json: Option<String>,
    /// HTTP method to execute
    method: Option<String>,
    /// Header parameters
    headers: Vec<String>,
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
            debug!("Request does not have a response, sending request");
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
    /// Takes the response from the request args
    pub fn take_response(&mut self) -> Option<Response<Incoming>> {
        self.response.take()
    }

    /// Creates a new request for url
    fn new(url: Url) -> Self {
        Self {
            url,
            use_http2: false,
            file: None,
            json: None,
            method: None,
            headers: vec![],
            response: None,
        }
    }

    /// Prepares the request to be called
    ///
    /// Returns an error if the binding doesn't match the plugin type, or if the body could not be set on the request
    async fn prepare(
        binding: &Bind<Self>,
    ) -> reality::Result<(DefaultClient, hyper::Request<Body>)> {
        let plugin = binding.plugin()?;
        let request = plugin
            .create_request()
            .await
            .map_err(|e| binding.plugin_call_error(e.to_string()))?;
        Ok((plugin.client(), request))
    }

    /// Creates a new default client
    #[inline]
    fn client(&self) -> DefaultClient {
        https(self.use_http2)
    }

    /// Creates the http request
    #[inline]
    async fn create_request(&self) -> hyper::http::Result<hyper::Request<Body>> {
        let url_authority = self.url.authority();
        let mut builder = RequestBuilder::new()
            .uri(self.url.to_string())
            .header(header::HOST, url_authority);

        if let Some(method) = self.method.as_ref() {
            match method.to_uppercase().as_str() {
                "PUT" => {
                    builder = builder.method("PUT");
                }
                "POST" => {
                    builder = builder.method("POST");
                }
                "PATCH" => {
                    builder = builder.method("PATCH");
                }
                "DELETE" => {
                    builder = builder.method("DELETE");
                }
                _ => {}
            }
        }

        self.finish_build(self.set_headers(builder)).await
    }

    /// Parse and set the headers for the request
    #[inline]
    fn set_headers(&self, mut builder: RequestBuilder) -> RequestBuilder {
        let headers = self.headers.join(";;");
        for (header, v) in reality::runir::util::scan_for_headers(&headers) {
            builder = builder.header(header.to_lowercase(), v.join(","));
        }
        builder
    }

    /// Finish building the request
    #[inline]
    async fn finish_build(
        &self,
        builder: RequestBuilder,
    ) -> hyper::http::Result<hyper::Request<Body>> {
        if let Some(json) = self.json.as_ref() {
            let body = StringBody::from(json.to_string()).into_boxed_body();
            builder
                .header(hyper::header::CONTENT_LENGTH, json.len())
                .header(
                    hyper::header::CONTENT_TYPE,
                    "application/json; charset=utf-8",
                )
                .body(body)
        } else if let Some(path) = self.file.as_ref() {
            if let Some(headers) = builder.headers_ref() {
                if headers
                    .get(hyper::header::CONTENT_TYPE)
                    .or(headers.get(hyper::header::CONTENT_DISPOSITION))
                    .is_none()
                {
                    warn!(
                        "A file was passed but neither Content-Type or Content-Disposition are set"
                    );
                }
            }
            let body = tokio::fs::read(path).await.unwrap();
            let body = BytesMut::from_iter(&body).freeze();
            builder
                .header(hyper::header::CONTENT_LENGTH, body.len())
                .body(BytesBody::from(body).into_boxed_body())
        } else {
            builder.body(EmptyBody.into_boxed_body())
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
    use std::ffi::OsString;

    use clap::{CommandFactory, Parser, Subcommand};
    use http_body_util::BodyExt;
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
            .load_by_toml::<Request>(
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
            .load_by_toml::<Request>(
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

    #[tokio::test]
    async fn test_request_plugin_call_from_engine_post_with_json() {
        let mut state = State::new();
        state
            .load_by_toml::<Request>(
                r#"
url = "https://jsonplaceholder.typicode.com/posts"
method = "POST"
json = """
{
  "title": "foo",
  "body": "bar",
  "userId": 1
}
"""
headers = [
  "x-ms-CORRELATION-ID=test"
]
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

        let mut resp = engine
            .state()
            .find_plugin("kioto/0.1.0/plugins/request")
            .map(|i| i.clone())
            .and_then(|mut i| i.borrow_mut::<Request>().and_then(|r| r.response.take()))
            .expect("should return resp");
        let incoming = resp.body_mut();
        let collected = incoming.collect().await.unwrap();
        eprintln!("{:#?}", collected);
        eprintln!("{}", String::from_utf8_lossy(&collected.to_bytes()));
        assert!(resp.status().is_success());
        ()
    }

    #[derive(Parser)]
    struct TestParser {
        #[clap(subcommand)]
        command: TestSubcommands,
    }

    #[derive(Subcommand)]
    enum TestSubcommands {
        Test(RequestArgs),
    }

    fn test_mock_request_args<'a, I, T>(args: I) -> super::Request
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut state = State::new();

        let matches = TestParser::command().try_get_matches_from(args).unwrap();

        let (_, subcommand) = matches.subcommand().unwrap();

        state
            .load_by_args::<RequestArgs>(subcommand)
            .expect("should be able to load request");

        let mut plugin = state
            .find_plugin("kioto/0.1.0/plugins/requestargs")
            .unwrap()
            .clone();
        let args = plugin.borrow_mut::<RequestArgs>().unwrap();
        args.take_request().expect("should have loaded a request")
    }

    #[tokio::test]
    async fn test_request_args_parse_args_plugin_loading_single_arg() {
        let req = test_mock_request_args([
            "testparser",
            "test",
            "https://jsonplaceholder.typicode.com/posts",
        ]);
        assert_eq!(
            "https://jsonplaceholder.typicode.com/posts",
            req.url.as_str()
        );
        assert!(!req.use_http2);
        assert!(req.file.is_none());
        assert!(req.json.is_none());
        assert!(req.headers.is_empty());
        assert!(req.response.is_none());
        assert!(req.method.is_none());
        ()
    }

    #[tokio::test]
    async fn test_request_args_parse_args_plugin_loading_put_method() {
        let req = test_mock_request_args([
            "testparser",
            "test",
            "--put",
            "https://jsonplaceholder.typicode.com/posts",
        ]);
        assert_eq!(
            "https://jsonplaceholder.typicode.com/posts",
            req.url.as_str()
        );
        assert!(!req.use_http2);
        assert!(req.file.is_none());
        assert!(req.json.is_none());
        assert!(req.headers.is_empty());
        assert!(req.response.is_none());
        assert_eq!("PUT", req.method.clone().unwrap());
        ()
    }

    #[tokio::test]
    async fn test_request_args_parse_args_plugin_loading_post_method() {
        let req = test_mock_request_args([
            "testparser",
            "test",
            "--post",
            "https://jsonplaceholder.typicode.com/posts",
        ]);
        assert_eq!(
            "https://jsonplaceholder.typicode.com/posts",
            req.url.as_str()
        );
        assert!(!req.use_http2);
        assert!(req.file.is_none());
        assert!(req.json.is_none());
        assert!(req.headers.is_empty());
        assert!(req.response.is_none());
        assert_eq!("POST", req.method.clone().unwrap());
        ()
    }

    #[tokio::test]
    async fn test_request_args_parse_args_plugin_loading_delete_method() {
        let req = test_mock_request_args([
            "testparser",
            "test",
            "--delete",
            "https://jsonplaceholder.typicode.com/posts",
        ]);
        assert_eq!(
            "https://jsonplaceholder.typicode.com/posts",
            req.url.as_str()
        );
        assert!(!req.use_http2);
        assert!(req.file.is_none());
        assert!(req.json.is_none());
        assert!(req.headers.is_empty());
        assert!(req.response.is_none());
        assert_eq!("DELETE", req.method.clone().unwrap());
        ()
    }

    #[tokio::test]
    async fn test_request_args_parse_args_plugin_loading_patch_method() {
        let req = test_mock_request_args([
            "testparser",
            "test",
            "--patch",
            "https://jsonplaceholder.typicode.com/posts",
        ]);
        assert_eq!(
            "https://jsonplaceholder.typicode.com/posts",
            req.url.as_str()
        );
        assert!(!req.use_http2);
        assert!(req.file.is_none());
        assert!(req.json.is_none());
        assert!(req.headers.is_empty());
        assert!(req.response.is_none());
        assert_eq!("PATCH", req.method.clone().unwrap());
        ()
    }

    #[tokio::test]
    async fn test_request_args_parse_args_plugin_loading_headers() {
        let req = test_mock_request_args([
            "testparser",
            "test",
            "--patch",
            "-H 'Accept=application/json; charset=UTF-8'",
            "-H 'Accept=application/json2'",
            "-H 'x-ms-Custom=test'",
            "https://jsonplaceholder.typicode.com/posts",
        ]);
        assert_eq!(
            "https://jsonplaceholder.typicode.com/posts",
            req.url.as_str()
        );
        assert!(!req.use_http2);
        assert!(req.file.is_none());
        assert!(req.json.is_none());
        assert_eq!(
            [
                "Accept=application/json; charset=UTF-8",
                "Accept=application/json2",
                "x-ms-Custom=test"
            ],
            &req.headers[..]
        );
        assert!(req.response.is_none());
        assert_eq!("PATCH", req.method.clone().unwrap());
        ()
    }
}
