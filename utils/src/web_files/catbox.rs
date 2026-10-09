//! Source code adapted from https://github.com/Savolae/catbox (MIT license) by Eetu Savolainen

use reqwest::multipart::{Form, Part};
use reqwest::{Body, Client};
use rootcause::report;
use std::path::Path;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

static LITTER_API_URL: &str = "https://litterbox.catbox.moe/resources/internals/api.php";
static UASTRING: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (archival tool for discord channels) - ",
    env!("CARGO_PKG_AUTHORS"),
);

pub async fn upload(file_path: impl AsRef<Path>, filename: String) -> rootcause::Result<String> {
    let file = Body::wrap_stream(ReaderStream::new(File::open(file_path).await?));

    let form = Form::new()
        .text("reqtype", "fileupload")
        .text("time", "1h")
        .part("fileToUpload", Part::stream(file).file_name(filename));

    Ok(Client::builder()
        .user_agent(UASTRING)
        .build()
        .unwrap_or_else(|_| Client::new())
        .post(LITTER_API_URL)
        .multipart(form)
        .send()
        .await?
        .text()
        .await?)
}
