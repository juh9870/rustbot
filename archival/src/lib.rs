use crate::archival::{archive_messages, ArchiveData};
use anyhow::Error;
use anyhow::{Context as AnyhowContext, Result};
use futures::TryStreamExt;
use poise::serenity_prelude::{ButtonStyle, Timestamp, UserId};
use std::time::Duration;
use tokio::time::sleep;
use utils::command_handler_wrapper;
use utils::component_tools::{clear_components, set_dummy_text_component};
use utils::confirmations::{confirm_buttons, BtnConfirmOptions};
use utils::messages_iter::{smart_messages_iter, MessagesRange};
use utils::web_files::messaged::upload_file_and_message;
use wiper::wiping::wipe_messages;

pub mod archival;

type Context<'a, T> = poise::Context<'a, T, Error>;

#[macro_export]
macro_rules! archive_command {
    ($name:ident, $data:ty) => {
        #[poise::command(
            slash_command,
            prefix_command,
            required_permissions = "MANAGE_MESSAGES",
            default_member_permissions = "MANAGE_MESSAGES",
            required_bot_permissions = "MANAGE_MESSAGES|READ_MESSAGE_HISTORY",
            guild_only
        )]
        async fn $name(
            ctx: poise::Context<'_, $data, anyhow::Error>,
            #[description = "Name of the archive"] archive_name: String,
        ) -> Result<()> {
            archival::archive(ctx, archive_name).await
        }
    };
}

pub async fn archive<T: Sync + Send>(ctx: Context<'_, T>, archive_name: String) -> Result<()> {
    command_handler_wrapper!(handle_archive(
        ctx,
        MessagesRange::unbounded(),
        archive_name,
    ))
}

async fn handle_archive<T: Sync + Send>(
    ctx: Context<'_, T>,
    mut messages_range: MessagesRange,
    mut archive_name: String,
) -> Result<()> {
    let mut reply = ctx
        .say("Are you sure you want to archive this channel?")
        .await?
        .into_message()
        .await?;

    let confirmed = confirm_buttons(
        ctx,
        &mut reply,
        BtnConfirmOptions {
            confirm_text: "Confirm archival".to_string(),
            confirm_style: ButtonStyle::Primary,
            cancel_text: "Cancel".to_string(),
            cancel_style: ButtonStyle::Secondary,
            timeout: Duration::from_secs(15),
        },
    )
    .await?
    .bool();

    if messages_range.before.is_none() {
        if let Context::Prefix(ctx) = ctx {
            messages_range.before = Some(ctx.msg.id);
        } else {
            messages_range.before = Some(reply.id);
        }
    }

    clear_components(ctx, &mut reply).await?;
    if !confirmed {
        reply
            .edit(ctx, |msg| msg.content("Operation canceled"))
            .await?;
        return Ok(());
    }
    reply
        .edit(ctx, |msg| msg.content("Archival in progress"))
        .await?;

    let response_id = reply.id;

    let ArchiveData { file, time_range } = archive_messages(
        ctx,
        smart_messages_iter(ctx, ctx.channel_id(), messages_range).map_err(|e| e.into()),
        |status| async {
            ctx.channel_id()
                .edit_message(ctx, response_id, |msg| msg.content(status))
                .await?;
            Ok(())
        },
    )
    .await?;

    let date_string = {
        let start_day = time_range.start.date_naive();
        let end_day = time_range.end.date_naive();
        if start_day == end_day {
            end_day.format("%Y-%m-%d").to_string()
        } else {
            format!(
                "{} to {}",
                start_day.format("%Y-%m-%d"),
                end_day.format("%Y-%m-%d")
            )
        }
    };

    // Assume user id
    if let Ok(id) = archive_name.parse::<u64>() {
        let user = UserId::from(id);
        if let Ok(user) = user.to_user(ctx).await {
            if user.discriminator == 0 {
                archive_name = format!("{} ({})", user.name, user.id)
            } else {
                archive_name = format!("{}#{:0>4} ({})", user.name, user.discriminator, user.id)
            }
        }
    }

    let filename = format!("{} - {archive_name}.zip", date_string);

    reply
        .edit(ctx, |msg| msg.content("Uploading archive"))
        .await?;

    let timeout = 60 * 15;

    let edit_prefix = format!(
        "Archival successful. Archive name: `{filename}`\nMessages deletion will automatically be canceled <t:{}:R>",
        Timestamp::now().unix_timestamp() + timeout
    );

    let mut latest_message = upload_file_and_message(
        ctx,
        ctx.channel_id(),
        file.as_file(),
        file.path(),
        filename.to_string(),
        edit_prefix,
    )
    .await?;

    reply.delete(ctx).await?;

    let confirmed = confirm_buttons(
        ctx,
        &mut latest_message,
        BtnConfirmOptions {
            confirm_text: "Wipe archived messages".to_string(),
            confirm_style: ButtonStyle::Danger,
            cancel_text: "Cancel".to_string(),
            cancel_style: ButtonStyle::Primary,
            timeout: Duration::from_secs(timeout as u64),
        },
    )
    .await?
    .bool();

    if confirmed {
        clear_components(ctx, &mut latest_message).await?;

        let wiper_status = ctx
            .say("Initializing wiper")
            .await?
            .into_message()
            .await?
            .id;
        let channel = ctx.channel_id();

        wipe_messages(
            ctx,
            smart_messages_iter(ctx, ctx.channel_id(), messages_range).map_err(|e| e.into()),
            |status, is_due| async move {
                if is_due {
                    channel
                        .edit_message(ctx, wiper_status, |msg| msg.content(status))
                        .await?;
                }
                Ok(())
            },
        )
        .await
        .context("wiping")?;

        sleep(Duration::from_secs(15)).await;

        let _ = channel.delete_message(ctx, wiper_status).await;
    } else {
        set_dummy_text_component(ctx, &mut latest_message, "Wiping canceled").await?;
    }

    file.close()?;

    Ok(())
}
