use anyhow::Context;
use poise::serenity_prelude::{ChannelId, CreateAttachment, CreateMessage, Message};
use std::fs::File;
use std::path::Path;

pub async fn upload_file_and_message<T: Sync + Send>(
    ctx: poise::Context<'_, T, anyhow::Error>,
    channel: ChannelId,
    file: &File,
    path: impl AsRef<Path>,
    filename: String,
    message_prefix: String,
) -> anyhow::Result<Message> {
    let size = file.metadata()?.len();
    // Less than 25 MB limit to be safe
    const SIZE_LIMIT: u64 = 1000 * 1000 * 24;
    let latest_message = if size < SIZE_LIMIT {
        let tokio_file = tokio::fs::File::open(path).await?;
        channel
            .send_files(
                ctx.serenity_context(),
                [CreateAttachment::file(&tokio_file, filename.as_str()).await?],
                CreateMessage::new().content(message_prefix),
            )
            .await
            .context("uploading archive to discord")?
    } else {
        let uploaded = super::upload_file(path, filename)
            .await
            .context("uploading archive to file.io")?;

        let content = CreateMessage::new().content(format!(
            "{message_prefix}\nDownload archive at {}\nFile will expire <t:{}:R>, or after {}",
            uploaded.link,
            uploaded.expires.unix_timestamp(),
            pluralizer::pluralize("downloads", uploaded.max_downloads, true)
        ));

        channel
            .send_message(ctx.serenity_context(), content)
            .await
            .context("sending link to discord")?
    };
    Ok(latest_message)
}
