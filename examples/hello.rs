use std::{
    collections::HashMap,
    convert::Infallible,
    env,
    fs::remove_file,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use async_stream::stream;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, RwLock},
    time::{interval, Duration},
};
use tokio_stream::wrappers::IntervalStream;

use viz::middleware::*;
use viz::prelude::*;
use viz::utils::{
    anyhow,
    futures::{pin_mut, FutureExt, StreamExt},
    serde::json,
    thiserror::Error as ThisError,
    tracing,
};

use redis::Client as RedisClient;

use fs::{Config as ServeConfig, Serve};
use jwt::jsonwebtoken;
use sse::*;
use ws::*;

const NOT_FOUND: &str = "404 - This is not the web page you are looking for.";

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, Error>>>>>;

async fn my_mid_error(cx: &mut Context) -> Result<Response> {
    if cx.path() == "/error" {
        bail!("my mid error")
    } else {
        cx.next().await
    }
}

async fn my_mid(cx: &mut Context) -> Result<Response> {
    let num = cx.extract::<State<Arc<AtomicUsize>>>().await?;

    num.as_ref().fetch_add(1, Ordering::SeqCst);

    tracing::info!("IN  Mid: {} {} - {:?}", cx.method(), cx.path(), &num);

    let num = cx.state::<Arc<AtomicUsize>>()?;

    num.as_ref().fetch_add(1, Ordering::SeqCst);

    // tracing::info!("IN  Mid: {} {} - {:?}", cx.method(), cx.path(), num);

    let fut = cx.next().await;

    tracing::info!("OUT Mid: {} {}", cx.method(), cx.path());

    Ok(match fut {
        Ok(mut res) => {
            if res.status() == http::StatusCode::NOT_FOUND {
                *res.body_mut() = NOT_FOUND.into();
            }

            res
        }
        Err(e) => {
            tracing::error!("middle error {}", e);
            e.into()
        }
    })
}

#[derive(ThisError, Debug)]
enum UserError {
    #[error("User Not Found")]
    NotFound,
}

impl Into<Response> for UserError {
    fn into(self) -> Response {
        (http::StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into()
    }
}

async fn not_found() -> http::StatusCode {
    tracing::info!("{:8}Exec: Not Found!", "");
    http::StatusCode::NOT_FOUND
}

async fn hello_world(num: State<Arc<AtomicUsize>>) -> String {
    num.as_ref().fetch_sub(1, Ordering::SeqCst);

    tracing::info!("{:8}Exec: Hello World! - {:?}", "", num);

    "Hello, World!".to_string()
}

// async fn server_error() -> Result<Response> {
async fn server_error() -> Result<Response, UserError> {
    Err(UserError::NotFound)
    // Err(how!(UserError::NotFound))
    // reject!(UserError::NotFound)
}

fn allow_get(cx: &Context) -> bool {
    tracing::info!("{:>8} Get: {}", "", cx.method() == http::Method::GET);
    cx.method() == http::Method::GET
}

fn allow_head(cx: &Context) -> bool {
    tracing::info!("{:>8}Head: {}", "", cx.method() == http::Method::HEAD);
    cx.method() == http::Method::HEAD
}

#[derive(Debug, Deserialize, Serialize)]
struct User {
    id: usize,
    name: String,
}

async fn create_user(user: Json<User>) -> Result<String> {
    dbg!(123);
    json::to_string_pretty(&*user).map_err(|e| anyhow!(e))
}

async fn login(session: State<session::Session>) -> Result<String> {
    session.set::<String>("name", String::from("viz"));
    session.save().await?;
    Ok("Session Logined".to_string())
}

async fn renew(mut session: State<session::Session>) -> Result<String> {
    session.renew().await?;
    Ok("Session Renewed".to_string())
}

async fn logout(session: State<session::Session>) -> Result<String> {
    session.destroy().await?;
    Ok("Session Logouted".to_string())
}

fn sse_counter(counter: u64) -> Result<Event, Infallible> {
    Ok(sse::Event::default().data(counter.to_string()))
}

async fn ticks() -> Response {
    let mut counter: u64 = 0;
    // create server event source
    let interval = interval(Duration::from_secs(1));
    let stream = IntervalStream::new(interval);
    let event_stream = stream.map(move |_| {
        counter += 1;
        sse_counter(counter)
    });
    // reply using server-sent events
    sse::reply(event_stream)
}

async fn echo(ws: Ws) -> Response {
    ws.on_upgrade(|websocket| {
        // Just echo all messages back...
        let (tx, rx) = websocket.split();
        rx.forward(tx).map(|result| {
            if let Err(e) = result {
                eprintln!("websocket error: {:?}", e);
            }
        })
    })
}

async fn chat(cx: &mut Context) -> Result<Response> {
    let users = cx.state::<Users>()?;
    Ok(match cx.ws() {
        Ok(ws) => ws.on_upgrade(move |socket| user_connected(socket, users)),
        Err(rs) => rs,
    })
}

async fn user_connected(ws: WebSocket, users: Users) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new chat user: {}", my_id);

    // Split the socket into a sender and receive of messages.
    let (user_ws_tx, mut user_ws_rx) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    let stream = stream! {
        pin_mut!(rx);
        while let Some(value) = rx.recv().await {
            yield value;
        }
    };
    tokio::task::spawn(stream.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));

    // Save the sender in our list of connected users.
    users.write().await.insert(my_id, tx);

    // Return a `Future` that is basically a state machine managing
    // this specific user's connection.

    // Make an extra clone to give to our disconnection handler...
    let users2 = users.clone();

    // Every time the user sends a message, broadcast it to
    // all other users...
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        user_message(my_id, msg, &users).await;
    }

    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    user_disconnected(my_id, &users2).await;
}

async fn user_message(my_id: usize, msg: Message, users: &Users) {
    // Skip any non-Text messages...
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };

    let new_msg = format!("<User#{}>: {}", my_id, msg);

    // New message from this user, send it to everyone else (except same uid)...
    for (&uid, tx) in users.read().await.iter() {
        if my_id != uid {
            if let Err(_disconnected) = tx.send(Ok(Message::text(new_msg.clone()))) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }
}

async fn user_disconnected(my_id: usize, users: &Users) {
    eprintln!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users.write().await.remove(&my_id);
}

static INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <title>Warp Chat</title>
    </head>
    <body>
        <h1>Warp chat</h1>
        <div id="chat">
            <p><em>Connecting...</em></p>
        </div>
        <input type="text" id="text" />
        <button type="button" id="send">Send</button>
        <script type="text/javascript">
        const chat = document.getElementById('chat');
        const text = document.getElementById('text');
        const uri = 'ws://' + location.host + '/chat/';
        const ws = new WebSocket(uri);
        function message(data) {
            const line = document.createElement('p');
            line.innerText = data;
            chat.appendChild(line);
        }
        ws.onopen = function() {
            chat.innerHTML = '<p><em>Connected!</em></p>';
        };
        ws.onmessage = function(msg) {
            message(msg.data);
        };
        ws.onclose = function() {
            chat.getElementsByTagName('em')[0].innerText = 'Disconnected!';
        };
        send.onclick = function() {
            const msg = text.value;
            ws.send(msg);
            text.value = '';
            message('<You>: ' + msg);
        };
        </script>
    </body>
</html>
"#;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        // From env var: `RUST_LOG`
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| anyhow::anyhow!(e));

    let mut app = viz::new();

    let config = app.config().await;

    dbg!(&config);

    let users = Users::default();

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        company: String,
    }

    let my_claims = Claims { sub: "hello".to_string(), company: "viz".to_string() };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &my_claims,
        &jsonwebtoken::EncodingKey::from_secret("secret".as_ref()),
    )?;

    dbg!(token);

    app.state(Arc::new(AtomicUsize::new(0))).state(users).routes(
        router()
            .mid(LoggerMiddleware::default())
            .mid(RecoverMiddleware::default())
            .mid(RequestIDMiddleware::default())
            .mid(TimeoutMiddleware::default())
            .mid(CookiesMiddleware::default())
            // .mid(
            //     jwt::JWTMiddleware::<Claims>::new().validation(jsonwebtoken::Validation {
            //         validate_exp: false,
            //         ..Default::default()
            //     }),
            // )
            // .mid(
            //     auth::BasicMiddleware::new().users(
            //         [("viz".to_string(), "rust".to_string())]
            //             .iter()
            //             .cloned()
            //             .collect(),
            //     ),
            // )
            // .mid(session::SessionMiddleware::new(session::Config {
            //     cookie: session::CookieOptions::new(),
            //     // storage: Arc::new(middleware::session::MemoryStorage::default()),
            //     storage: Arc::new(session::RedisStorage::new(RedisClient::open(
            //         "redis://127.0.0.1",
            //     )?)),
            //     generate: Box::new(|| nano_id::base64(32)),
            //     verify: Box::new(|sid: &str| sid.len() == 32),
            // }))
            // .mid(compression::brotli())
            .mid(my_mid_error)
            .mid(my_mid)
            .at(
                "/",
                route()
                    // .guard(allow_get)
                    .guard(
                        <Box<dyn Guard>>::from(allow_get)
                            | Into::<Box<dyn Guard>>::into(allow_head),
                    )
                    .all(hello_world),
            )
            .at("/users", route().post(create_user))
            .at("/login", route().post(login))
            .at("/renew", route().post(renew))
            .at("/logout", route().get(logout))
            .at("/404", route().all(server_error))
            .at("/ticks", route().get(ticks))
            .at("/echo", route().get(echo))
            .at("/chat", route().get(|| async { Response::html(INDEX_HTML) }))
            .at("/chat/", route().get2(chat))
            .at("/public/*", route().all3(Serve::new(ServeConfig::new(config.dir.join("public")))))
            .at("/*", route().all(not_found)),
    );

    //app.listen("127.0.0.1:8080").await

    let path = "tmp.sock";
    let _ = remove_file(path);

    app.listen_uds(path).await
}
