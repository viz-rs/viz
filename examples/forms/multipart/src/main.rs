use futures_util::TryStreamExt;
use std::{fs::File, net::SocketAddr};
use tempfile::tempdir;
use tokio::net::TcpListener;
use viz::{
    IntoHandler, IntoResponse, Request, Response, ResponseExt, Result, Router,
    middleware::limits,
    serve,
    types::{Multipart, PayloadError},
};

// HTML form for uploading photos
async fn new(_: Request) -> Result<Response> {
    Ok(Response::html(include_str!("../index.html")))
}

// upload photos
async fn upload(mut form: Multipart) -> Result<Response> {
    let dir = tempdir()?;

    let mut group = None;

    while let Some(mut field) = form.try_next().await? {
        if let Some(ref filename) = field.filename {
            let path = dir.path().join(filename);
            let mut file = File::create(&path)?;
            field.copy_to_file(&mut file).await?;
        } else {
            let buf = field.bytes().await?;
            group.replace(String::from_utf8(buf.to_vec()).map_err(PayloadError::Utf8)?);
        }
    }

    // clean the dir
    dir.close()?;

    Ok(match group {
        Some(group) => group.into_response(),
        None => "Default".into_response(),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("listening on http://{addr}");

    let app = Router::new()
        .get("/", new)
        .post("/", upload.into_handler())
        // limit body size
        .with(limits::Config::default());

    if let Err(e) = serve(listener, app).await {
        println!("{e}");
    }

    Ok(())
}
