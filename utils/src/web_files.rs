use poise::serenity_prelude::Timestamp;
use rootcause::prelude::ResultExt;
use serde::Deserialize;
use std::path::Path;
use tokio::io::AsyncWriteExt;

mod catbox;
pub mod messaged;

pub async fn upload_file(path: impl AsRef<Path>, filename: String) -> rootcause::Result<String> {
    catbox::upload(path, filename).await
}

pub async fn download_to_file(url: &str, file: &Path) -> rootcause::Result<()> {
    let mut retries = 3;
    let bytes = loop {
        let response = reqwest::get(url).await?;
        let bytes = response
            .bytes()
            .await
            .context_with(|| format!("downloading file at {url}"));
        match bytes {
            Ok(bytes) => break bytes,
            Err(err) => {
                if retries == 0 {
                    return Err(err.into());
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
