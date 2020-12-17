//! WebSockets
//! Thanks: https://github.com/seanmonstar/warp

use std::{
    borrow::Cow,
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use headers::{
    Connection, HeaderMapExt, SecWebsocketAccept, SecWebsocketKey, SecWebsocketVersion, Upgrade,
};
use tokio_tungstenite::{
    tungstenite::protocol::{self, WebSocketConfig},
    WebSocketStream,
};

use viz_utils::{
    futures::{
        future::{self, BoxFuture, FutureExt, TryFutureExt},
        ready,
        sink::Sink,
        stream::Stream,
    },
    log,
};

use crate::{Error, Extract};

/// Context Extends
pub trait WsContextExt {
    fn ws(&mut self) -> Result<Ws, crate::Response>;
}

impl WsContextExt for crate::Context {
    fn ws(&mut self) -> Result<Ws, crate::Response> {
        let headers = self.headers();
        headers
            .typed_get::<Upgrade>()
            .filter(|upgrade| upgrade == &Upgrade::websocket())
            .and(headers.typed_get::<Connection>())
            .filter(|connection| connection.contains(http::header::UPGRADE))
            .and(headers.typed_get::<SecWebsocketVersion>())
            .filter(|version| version == &SecWebsocketVersion::V13)
            .and(headers.typed_get::<SecWebsocketKey>())
            .zip(self.take_body())
            .map(|(key, body)| Ws {
                key,
                body,
                config: None,
            })
            .ok_or_else(|| {
                (
                    http::StatusCode::BAD_REQUEST,
                    "invalid websocket upgrade request",
                )
                    .into()
            })
    }
}

impl Extract for Ws {
    type Error = crate::Response;

    #[inline]
    fn extract<'a>(cx: &'a mut crate::Context) -> BoxFuture<'a, Result<Self, Self::Error>> {
        Box::pin(async move { cx.ws() })
    }
}

/// Extracted by the [`ws`](ws) filter, and used to finish an upgrade.
pub struct Ws {
    body: ::hyper::Body,
    config: Option<WebSocketConfig>,
    key: SecWebsocketKey,
}

impl Ws {
    /// Finish the upgrade, passing a function to handle the `WebSocket`.
    ///
    /// The passed function must return a `Future`.
    pub fn on_upgrade<F, U>(self, func: F) -> crate::Response
    where
        F: FnOnce(WebSocket) -> U + Send + 'static,
        U: Future<Output = ()> + Send + 'static,
    {
        WsResponse {
            ws: self,
            on_upgrade: func,
        }
        .into()
    }

    // config

    /// Set the size of the internal message send queue.
    pub fn max_send_queue(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(WebSocketConfig::default)
            .max_send_queue = Some(max);
        self
    }

    /// Set the maximum message size (defaults to 64 megabytes)
    pub fn max_message_size(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(WebSocketConfig::default)
            .max_message_size = Some(max);
        self
    }

    /// Set the maximum frame size (defaults to 16 megabytes)
    pub fn max_frame_size(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(|| WebSocketConfig::default())
            .max_frame_size = Some(max);
        self
    }
}

impl fmt::Debug for Ws {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ws").finish()
    }
}

#[allow(missing_debug_implementations)]
struct WsResponse<F> {
    ws: Ws,
    on_upgrade: F,
}

impl<F, U> From<WsResponse<F>> for crate::Response
where
    F: FnOnce(WebSocket) -> U + Send + 'static,
    U: Future<Output = ()> + Send + 'static,
{
    fn from(v: WsResponse<F>) -> crate::Response {
        let on_upgrade = v.on_upgrade;
        let config = v.ws.config;

        let fut =
            v.ws.body
                .on_upgrade()
                .and_then(move |upgraded| {
                    log::trace!("websocket upgrade complete");
                    WebSocket::from_raw_socket(upgraded, protocol::Role::Server, config).map(Ok)
                })
                .and_then(move |socket| on_upgrade(socket).map(Ok))
                .map(|result| {
                    if let Err(err) = result {
                        log::debug!("ws upgrade error: {}", err);
                    }
                });

        ::tokio::task::spawn(fut);

        let mut res = http::Response::default();

        *res.status_mut() = http::StatusCode::SWITCHING_PROTOCOLS;

        res.headers_mut().typed_insert(Connection::upgrade());
        res.headers_mut().typed_insert(Upgrade::websocket());
        res.headers_mut()
            .typed_insert(SecWebsocketAccept::from(v.ws.key));

        res.into()
    }
}

/// A websocket `Stream` and `Sink`, provided to `ws` filters.
///
/// Ping messages sent from the client will be handled internally by replying with a Pong message.
/// Close messages need to be handled explicitly: usually by closing the `Sink` end of the
/// `WebSocket`.
pub struct WebSocket {
    inner: WebSocketStream<hyper::upgrade::Upgraded>,
}

impl WebSocket {
    pub(crate) async fn from_raw_socket(
        upgraded: hyper::upgrade::Upgraded,
        role: protocol::Role,
        config: Option<protocol::WebSocketConfig>,
    ) -> Self {
        WebSocketStream::from_raw_socket(upgraded, role, config)
            .map(|inner| WebSocket { inner })
            .await
    }

    /// Gracefully close this websocket.
    pub async fn close(mut self) -> Result<(), Error> {
        future::poll_fn(|cx| Pin::new(&mut self).poll_close(cx)).await
    }
}

impl Stream for WebSocket {
    type Item = Result<Message, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(Pin::new(&mut self.inner).poll_next(cx)) {
            Some(Ok(item)) => Poll::Ready(Some(Ok(Message { inner: item }))),
            Some(Err(e)) => {
                log::debug!("websocket poll error: {}", e);
                Poll::Ready(Some(Err(Error::new(e))))
            }
            None => {
                log::trace!("websocket closed");
                Poll::Ready(None)
            }
        }
    }
}

impl Sink<Message> for WebSocket {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_ready(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(Error::new(e))),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match Pin::new(&mut self.inner).start_send(item.inner) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::debug!("websocket start_send error: {}", e);
                Err(Error::new(e))
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_flush(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(Error::new(e))),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_close(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(err) => {
                log::debug!("websocket close error: {}", err);
                Poll::Ready(Err(Error::new(err)))
            }
        }
    }
}

impl fmt::Debug for WebSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebSocket").finish()
    }
}

/// A WebSocket message.
///
/// This will likely become a `non-exhaustive` enum in the future, once that
/// language feature has stabilized.
#[derive(Eq, PartialEq, Clone)]
pub struct Message {
    inner: protocol::Message,
}

impl Message {
    /// Construct a new Text `Message`.
    pub fn text<S: Into<String>>(s: S) -> Message {
        Message {
            inner: protocol::Message::text(s),
        }
    }

    /// Construct a new Binary `Message`.
    pub fn binary<V: Into<Vec<u8>>>(v: V) -> Message {
        Message {
            inner: protocol::Message::binary(v),
        }
    }

    /// Construct a new Ping `Message`.
    pub fn ping<V: Into<Vec<u8>>>(v: V) -> Message {
        Message {
            inner: protocol::Message::Ping(v.into()),
        }
    }

    /// Construct the default Close `Message`.
    pub fn close() -> Message {
        Message {
            inner: protocol::Message::Close(None),
        }
    }

    /// Construct a Close `Message` with a code and reason.
    pub fn close_with(code: impl Into<u16>, reason: impl Into<Cow<'static, str>>) -> Message {
        Message {
            inner: protocol::Message::Close(Some(protocol::frame::CloseFrame {
                code: protocol::frame::coding::CloseCode::from(code.into()),
                reason: reason.into(),
            })),
        }
    }

    /// Returns true if this message is a Text message.
    pub fn is_text(&self) -> bool {
        self.inner.is_text()
    }

    /// Returns true if this message is a Binary message.
    pub fn is_binary(&self) -> bool {
        self.inner.is_binary()
    }

    /// Returns true if this message a is a Close message.
    pub fn is_close(&self) -> bool {
        self.inner.is_close()
    }

    /// Returns true if this message is a Ping message.
    pub fn is_ping(&self) -> bool {
        self.inner.is_ping()
    }

    /// Returns true if this message is a Pong message.
    pub fn is_pong(&self) -> bool {
        self.inner.is_pong()
    }

    /// Try to get a reference to the string text, if this is a Text message.
    pub fn to_str(&self) -> Result<&str, ()> {
        match self.inner {
            protocol::Message::Text(ref s) => Ok(s),
            _ => Err(()),
        }
    }

    /// Return the bytes of this message, if the message can contain data.
    pub fn as_bytes(&self) -> &[u8] {
        match self.inner {
            protocol::Message::Text(ref s) => s.as_bytes(),
            protocol::Message::Binary(ref v) => v,
            protocol::Message::Ping(ref v) => v,
            protocol::Message::Pong(ref v) => v,
            protocol::Message::Close(_) => &[],
        }
    }

    /// Destructure this message into binary data.
    pub fn into_bytes(self) -> Vec<u8> {
        self.inner.into_data()
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl Into<Vec<u8>> for Message {
    fn into(self) -> Vec<u8> {
        self.into_bytes()
    }
}
