use anyhow::{bail, Context};
use poise::serenity_prelude::Timestamp;
use serde::Deserialize;
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub mod messaged;

#[must_use]
#[derive(Debug, Deserialize)]
pub struct FileIoResponse {
    pub success: bool,
    pub key: String,
    pub link: String,
    pub expires: Timestamp,
    #[serde(rename = "maxDownloads")]
    pub max_downloads: isize,
}

pub async fn upload_file(
    path: impl AsRef<Path>,
    filename: String,
) -> anyhow::Result<FileIoResponse> {
    let content = tokio::fs::read(path.as_ref()).await?;
    let part = reqwest::multipart::Part::bytes(content).file_name(filename);
    let file = reqwest::multipart::Form::new().part("file", part);
    let response = reqwest::Client::new()
        .post("https://file.io")
        .multipart(file)
        .send()
        .await?;

    let response: FileIoResponse = response.json().await?;
    if !response.success {
        bail!("Upload resulted in a failure");
    }
    Ok(response)
}

pub async fn download_to_file(url: &str, file: &Path) -> anyhow::Result<()> {
    let mut retries = 3;
    let bytes = loop {
        let response = reqwest::get(url).await?;
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("downloading file at {url}"));
        match bytes {
            Ok(bytes) => break bytes,
            Err(err) => {
                if retries == 0 {
                    return Err(err);
                }
                retries -= 1;
            }
        }
    };
    tokio::fs::File::create(&file)
        .await?
        .write_all(&bytes)
        .await?;
    Ok(())
}
