//! Server-Sent Events (SSE)
//! Thanks: https://github.com/seanmonstar/warp

use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{self, Display, Formatter, Write},
    future::Future,
    pin::Pin,
    str::FromStr,
    task::{Context, Poll},
    time::Duration,
};

use http::header::{HeaderValue, CACHE_CONTROL, CONTENT_TYPE};
use pin_project::pin_project;
use serde::Serialize;
use tokio::time::{self, Delay};

use viz_utils::{
    futures::{
        future::{self, BoxFuture, FutureExt, TryFutureExt},
        ready,
        sink::Sink,
        stream::{Stream, TryStream, TryStreamExt},
    },
    log,
    serde::json as serde_json,
};

use self::sealed::{
    BoxedServerSentEvent, EitherServerSentEvent, SseError, SseField, SseFormat, SseWrapper,
};

/// Server-sent event message
pub trait ServerSentEvent: SseFormat + Sized + Send + Sync + 'static {
    /// Convert to either A
    fn into_a<B>(self) -> EitherServerSentEvent<Self, B> {
        EitherServerSentEvent::A(self)
    }

    /// Convert to either B
    fn into_b<A>(self) -> EitherServerSentEvent<A, Self> {
        EitherServerSentEvent::B(self)
    }

    /// Convert to boxed
    fn boxed(self) -> BoxedServerSentEvent {
        BoxedServerSentEvent(Box::new(self))
    }
}

impl<T: SseFormat + Send + Sync + 'static> ServerSentEvent for T {}

#[allow(missing_debug_implementations)]
struct SseComment<T>(T);

/// Comment field (":<comment-text>")
pub fn comment<T>(comment: T) -> impl ServerSentEvent
where
    T: Display + Send + Sync + 'static,
{
    SseComment(comment)
}

impl<T: Display> SseFormat for SseComment<T> {
    fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
        if let SseField::Comment = k {
            k.fmt(f)?;
            self.0.fmt(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseEvent<T>(T);

/// Event name field ("event:<event-name>")
pub fn event<T>(event: T) -> impl ServerSentEvent
where
    T: Display + Send + Sync + 'static,
{
    SseEvent(event)
}

impl<T: Display> SseFormat for SseEvent<T> {
    fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
        if let SseField::Event = k {
            k.fmt(f)?;
            self.0.fmt(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseId<T>(T);

/// Identifier field ("id:<identifier>")
pub fn id<T>(id: T) -> impl ServerSentEvent
where
    T: Display + Send + Sync + 'static,
{
    SseId(id)
}

impl<T: Display> SseFormat for SseId<T> {
    fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
        if let SseField::Id = k {
            k.fmt(f)?;
            self.0.fmt(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseRetry(Duration);

/// Retry timeout field ("retry:<timeout>")
pub fn retry(time: Duration) -> impl ServerSentEvent {
    SseRetry(time)
}

impl SseFormat for SseRetry {
    fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
        if let SseField::Retry = k {
            k.fmt(f)?;

            let secs = self.0.as_secs();
            let millis = self.0.subsec_millis();

            if secs > 0 {
                // format seconds
                secs.fmt(f)?;

                // pad milliseconds
                if millis < 10 {
                    f.write_str("00")?;
                } else if millis < 100 {
                    f.write_char('0')?;
                }
            }

            // format milliseconds
            millis.fmt(f)?;

            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseData<T>(T);

/// Data field(s) ("data:<content>")
///
/// The multiline content will be transferred
/// using sequential data fields, one per line.
pub fn data<T>(data: T) -> impl ServerSentEvent
where
    T: Display + Send + Sync + 'static,
{
    SseData(data)
}

impl<T: Display> SseFormat for SseData<T> {
    fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
        if let SseField::Data = k {
            for line in self.0.to_string().split('\n') {
                k.fmt(f)?;
                line.fmt(f)?;
                f.write_char('\n')?;
            }
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseJson<T>(T);

/// Data field with JSON content ("data:<json-content>")
pub fn json<T>(data: T) -> impl ServerSentEvent
where
    T: Serialize + Send + Sync + 'static,
{
    SseJson(data)
}

impl<T: Serialize> SseFormat for SseJson<T> {
    fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
        if let SseField::Data = k {
            k.fmt(f)?;
            serde_json::to_string(&self.0)
                .map_err(|error| {
                    log::error!("sse::json error {}", error);
                    fmt::Error
                })
                .and_then(|data| data.fmt(f))?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

macro_rules! tuple_fmt {
    (($($t:ident),+) => ($($i:tt),+)) => {
        impl<$($t),+> SseFormat for ($($t),+)
        where
            $($t: SseFormat,)+
        {
            fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
                $(self.$i.fmt_field(f, k)?;)+
                Ok(())
            }
        }
    };
}

tuple_fmt!((A, B) => (0, 1));
tuple_fmt!((A, B, C) => (0, 1, 2));
tuple_fmt!((A, B, C, D) => (0, 1, 2, 3));
tuple_fmt!((A, B, C, D, E) => (0, 1, 2, 3, 4));
tuple_fmt!((A, B, C, D, E, F) => (0, 1, 2, 3, 4, 5));
tuple_fmt!((A, B, C, D, E, F, G) => (0, 1, 2, 3, 4, 5, 6));
tuple_fmt!((A, B, C, D, E, F, G, H) => (0, 1, 2, 3, 4, 5, 6, 7));

/// Gets the optional last event id from request.
/// Typically this identifier represented as number or string.
/// Context Extends
pub trait SseContextExt {
    fn last_event_id<T>(&self) -> Option<T>
    where
        T: FromStr + Send + Sync + 'static;
}

impl SseContextExt for crate::Context {
    fn last_event_id<T>(&self) -> Option<T>
    where
        T: FromStr + Send + Sync + 'static,
    {
        self.header("last-event-id")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<T>().ok())
    }
}

/// Server-sent events reply
///
/// This function converts stream of server events into a `Reply` with:
///
/// - Status of `200 OK`
/// - Header `content-type: text/event-stream`
/// - Header `cache-control: no-cache`.
pub fn reply<S>(event_stream: S) -> crate::Response
where
    S: TryStream + Send + 'static,
    S::Ok: ServerSentEvent,
    S::Error: StdError + Send + Sync + 'static,
{
    SseResponse { event_stream }.into()
}

#[allow(missing_debug_implementations)]
struct SseResponse<S> {
    event_stream: S,
}

impl<S> From<SseResponse<S>> for crate::Response
where
    S: TryStream + Send + 'static,
    S::Ok: ServerSentEvent,
    S::Error: StdError + Send + Sync + 'static,
{
    #[inline]
    fn from(v: SseResponse<S>) -> crate::Response {
        let body_stream = v
            .event_stream
            .map_err(|error| {
                // FIXME: error logging
                log::error!("sse stream error: {}", error);
                SseError
            })
            .into_stream()
            .and_then(|event| future::ready(SseWrapper::format(&event)));

        let mut res = hyper::Response::new(hyper::Body::wrap_stream(body_stream));
        // Set appropriate content type
        res.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
        // Disable response body caching
        res.headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        res.into()
    }
}

/// Configure the interval between keep-alive messages, the content
/// of each message, and the associated stream.
#[derive(Debug)]
pub struct KeepAlive {
    comment_text: Cow<'static, str>,
    max_interval: Duration,
}

impl KeepAlive {
    /// Customize the interval between keep-alive messages.
    ///
    /// Default is 15 seconds.
    pub fn interval(mut self, time: Duration) -> Self {
        self.max_interval = time;
        self
    }

    /// Customize the text of the keep-alive message.
    ///
    /// Default is an empty comment.
    pub fn text(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.comment_text = text.into();
        self
    }

    /// Wrap an event stream with keep-alive functionality.
    ///
    /// See [`keep_alive`](keep_alive) for more.
    pub fn stream<S>(
        self,
        event_stream: S,
    ) -> impl TryStream<
        Ok = impl ServerSentEvent + Send + 'static,
        Error = impl StdError + Send + Sync + 'static,
    > + Send
           + 'static
    where
        S: TryStream + Send + 'static,
        S::Ok: ServerSentEvent + Send,
        S::Error: StdError + Send + Sync + 'static,
    {
        let alive_timer = time::delay_for(self.max_interval);
        SseKeepAlive {
            event_stream,
            comment_text: self.comment_text,
            max_interval: self.max_interval,
            alive_timer,
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
struct SseKeepAlive<S> {
    #[pin]
    event_stream: S,
    comment_text: Cow<'static, str>,
    max_interval: Duration,
    alive_timer: Delay,
}

/// Keeps event source connection alive when no events sent over a some time.
///
/// Some proxy servers may drop HTTP connection after a some timeout of inactivity.
/// This function helps to prevent such behavior by sending comment events every
/// `keep_interval` of inactivity.
///
/// By default the comment is `:` (an empty comment) and the time interval between
/// events is 15 seconds. Both may be customized using the builder pattern
/// as shown below link.
/// See [notes](https://html.spec.whatwg.org/multipage/server-sent-events.html).
pub fn keep_alive() -> KeepAlive {
    KeepAlive {
        comment_text: Cow::Borrowed(""),
        max_interval: Duration::from_secs(15),
    }
}

impl<S> Stream for SseKeepAlive<S>
where
    S: TryStream + Send + 'static,
    S::Ok: ServerSentEvent,
    S::Error: StdError + Send + Sync + 'static,
{
    type Item = Result<EitherServerSentEvent<S::Ok, SseComment<Cow<'static, str>>>, SseError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut pin = self.project();
        match pin.event_stream.try_poll_next(cx) {
            Poll::Pending => match Pin::new(&mut pin.alive_timer).poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(_) => {
                    // restart timer
                    pin.alive_timer
                        .reset(tokio::time::Instant::now() + *pin.max_interval);
                    let comment_str = pin.comment_text.clone();
                    Poll::Ready(Some(Ok(EitherServerSentEvent::B(SseComment(comment_str)))))
                }
            },
            Poll::Ready(Some(Ok(event))) => {
                // restart timer
                pin.alive_timer
                    .reset(tokio::time::Instant::now() + *pin.max_interval);
                Poll::Ready(Some(Ok(EitherServerSentEvent::A(event))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(error))) => {
                log::error!("sse::keep error: {}", error);
                Poll::Ready(Some(Err(SseError)))
            }
        }
    }
}

mod sealed {
    use super::*;

    /// SSE error type
    #[derive(Debug)]
    pub struct SseError;

    impl Display for SseError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "sse error")
        }
    }

    impl StdError for SseError {}

    impl Display for SseField {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            use self::SseField::*;
            f.write_str(match self {
                Event => "event:",
                Id => "id:",
                Data => "data:",
                Retry => "retry:",
                Comment => ":",
            })
        }
    }

    /// SSE field kind
    #[allow(missing_debug_implementations)]
    pub enum SseField {
        /// Event name field
        Event,
        /// Event id field
        Id,
        /// Event data field
        Data,
        /// Retry timeout field
        Retry,
        /// Comment field
        Comment,
    }

    /// SSE formatter trait
    pub trait SseFormat {
        /// format message field
        fn fmt_field(&self, _f: &mut Formatter<'_>, _key: &SseField) -> fmt::Result {
            Ok(())
        }
    }

    /// SSE wrapper to help formatting messages
    #[allow(missing_debug_implementations)]
    pub struct SseWrapper<'a, T: 'a>(&'a T);

    impl<'a, T> SseWrapper<'a, T>
    where
        T: SseFormat + 'a,
    {
        pub fn format(event: &'a T) -> Result<String, SseError> {
            let mut buf = String::new();
            buf.write_fmt(format_args!("{}", SseWrapper(event)))
                .map_err(|_| SseError)?;
            buf.shrink_to_fit();
            Ok(buf)
        }
    }

    impl<'a, T> Display for SseWrapper<'a, T>
    where
        T: SseFormat,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            self.0.fmt_field(f, &SseField::Comment)?;
            // The event name usually transferred before the other fields.
            self.0.fmt_field(f, &SseField::Event)?;
            // It is important that the data will be transferred before
            // the identifier to prevent possible losing events when
            // resuming connection.
            self.0.fmt_field(f, &SseField::Data)?;
            self.0.fmt_field(f, &SseField::Id)?;
            self.0.fmt_field(f, &SseField::Retry)?;
            f.write_char('\n')
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct BoxedServerSentEvent(pub(super) Box<dyn SseFormat + Send + Sync>);

    impl SseFormat for BoxedServerSentEvent {
        fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
            self.0.fmt_field(f, k)
        }
    }

    #[allow(missing_debug_implementations)]
    pub enum EitherServerSentEvent<A, B> {
        A(A),
        B(B),
    }

    impl<A, B> SseFormat for EitherServerSentEvent<A, B>
    where
        A: SseFormat,
        B: SseFormat,
    {
        fn fmt_field(&self, f: &mut Formatter<'_>, k: &SseField) -> fmt::Result {
            match self {
                EitherServerSentEvent::A(a) => a.fmt_field(f, k),
                EitherServerSentEvent::B(b) => b.fmt_field(f, k),
            }
        }
    }
}
