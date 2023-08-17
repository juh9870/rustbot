use anyhow::{bail, Context};
use serde::Deserialize;
use std::io::{Read, Write};
use std::path::Path;
use poise::serenity_prelude::Timestamp;
use walkdir::WalkDir;

pub fn archive_directory(dir: &Path, out_file: &mut std::fs::File) -> anyhow::Result<()> {
    let options = zip::write::FileOptions::default().unix_permissions(0o755);
    let mut zip = zip::ZipWriter::new(out_file);
    let mut buffer = Vec::new();
    for entry in WalkDir::new(&dir).into_iter() {
        let entry = entry.context("Serializing files")?;
        let path = entry.path();
        let name = path
            .strip_prefix(&dir)?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Bad file name"))?;

        if path.is_file() {
            zip.start_file(name, options)?;
            let mut f = std::fs::File::open(path)?;
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.is_empty() {
            zip.add_directory(name, options)?;
        }
    }
    Ok(())
}

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
