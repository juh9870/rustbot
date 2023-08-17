use anyhow::Result;
use anyhow::{anyhow, bail, Context as AnyhowContext, Error};

use crate::utils::{archive_directory, upload_file};
use pluralizer::pluralize;
use poise::futures_util::StreamExt;
use poise::serenity_prelude::{
    ButtonStyle, ComponentInteractionCollectorBuilder, EmojiId, GatewayIntents,
    InteractionResponseType, Message, ReactionType, Timestamp, User, UserId,
};
use poise::PrefixFrameworkOptions;
use poise::{serenity_prelude as serenity, Modal};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::{tempdir, NamedTempFile, TempDir};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;
use twemoji_assets::png::PngTwemojiAsset;

mod messages_iter;
mod utils;
mod wiping;

struct Data {}

type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug, Modal)]
#[name = "Are you sure you want to archive this channel"]
struct Confirmation {
    #[name = "Input \"archive\" to confirm."]
    #[placeholder = "This will wipe the channel, and cannot be undone."]
    confirmation: String,
}

async fn confirm(ctx: Context<'_>) -> Result<bool> {
    let result = match ctx {
        Context::Application(ctx) => Confirmation::execute(ctx)
            .await?
            .map(|e| e.confirmation)
            .unwrap_or("".to_string()),
        Context::Prefix(ctx) => {
            ctx.say("Are you sure you want to archive and wipe a channel?\nInput \"archive\" to confirm. This cannot be undone.").await?;
            let response = serenity::collector::MessageCollectorBuilder::new(ctx.serenity_context)
                .author_id(ctx.author().id)
                .channel_id(ctx.channel_id())
                .collect_limit(1)
                .timeout(Duration::from_secs(20))
                .build()
                .next()
                .await;
            response
                .map(|msg| msg.content.clone())
                .unwrap_or("".to_string())
        }
    };
    return Ok(result.trim().to_lowercase() == "archive");
}

#[derive(Debug)]
struct ArchivalState {
    root_dir: TempDir,
    assets_dir: PathBuf,
    file: File,
    processed_count: usize,
    avatars: FxHashMap<UserId, PathBuf>,
    emojis: FxHashMap<ReactionType, PathBuf>,
}

impl ArchivalState {
    async fn create() -> Result<Self> {
        let dir = tempdir()?;
        let file = File::create(dir.path().join("messages.json")).await?;
        let assets_dir = dir.path().join("assets");
        tokio::fs::create_dir(&assets_dir).await?;
        Ok(ArchivalState {
            root_dir: dir,
            assets_dir,
            file,
            processed_count: 0,
            avatars: Default::default(),
            emojis: Default::default(),
        })
    }

    async fn finalize(&mut self) -> Result<()> {
        self.file.write_all("\n]".as_bytes()).await?;
        Ok(())
    }
}

fn get_extension_from_url(file_url: &str) -> Result<String> {
    let parsed = url::Url::parse(file_url)?;
    Path::extension(parsed.path().as_ref())
        .and_then(|e| e.to_str())
        .map(|e| e.to_string())
        .ok_or_else(|| anyhow!("Missing file extension"))
}

async fn download_to_file(url: &str, file: &Path) -> Result<()> {
    let response = reqwest::get(url).await?;
    File::create(&file)
        .await?
        .write_all(
            &response
                .bytes()
                .await
                .with_context(|| format!("downloading file at {url}"))?,
        )
        .await?;
    Ok(())
}

async fn ensure_user_avatar<'a>(state: &'a mut ArchivalState, user: &mut User) -> Result<&'a Path> {
    if let std::collections::hash_map::Entry::Vacant(e) = state.avatars.entry(user.id) {
        let avatar_url = user.face();

        let extension = get_extension_from_url(&avatar_url)?;
        let file_path = state.assets_dir.join(format!("{}.{}", user.id, extension));

        download_to_file(&avatar_url, &file_path).await?;

        e.insert(file_path.strip_prefix(&state.root_dir)?.to_path_buf());
    }
    let avatar = state
        .avatars
        .get(&user.id)
        .expect("Failed to retrieve avatar reference");
    user.avatar = Some(
        avatar
            .to_str()
            .ok_or_else(|| anyhow!("Failed to stringify avatar url"))?
            .to_string(),
    );
    Ok(avatar)
}
async fn ensure_emoji<'a>(
    state: &'a mut ArchivalState,
    reaction: &ReactionType,
) -> Result<&'a Path> {
    if let std::collections::hash_map::Entry::Vacant(e) = state.emojis.entry(reaction.clone()) {
        match &reaction {
            ReactionType::Custom { id, .. } => {
                let url = format!("https://cdn.discordapp.com/emojis/{id}.png");
                let file_path = state.assets_dir.join(format!("{id}.png"));
                download_to_file(&url, &file_path).await?;
                e.insert(file_path.strip_prefix(&state.root_dir)?.to_path_buf());
            }
            ReactionType::Unicode(emoji) => {
                println!("{emoji}");
                let asset = PngTwemojiAsset::from_emoji(emoji)
                    .or_else(|| PngTwemojiAsset::from_emoji(&emoji[..1]));
                if let Some(asset) = asset {
                    let file_name = state.assets_dir.join(format!(
                        "{}.png",
                        asset
                            .label
                            .ok_or_else(|| anyhow!("Failed to get emoji label"))?
                    ));
                    File::create(&file_name)
                        .await?
                        .write_all(asset.data.0)
                        .await?;
                    e.insert(file_name.strip_prefix(&state.root_dir)?.to_path_buf());
                } else {
                    let file_name = state.assets_dir.join("broken_emoji.png");
                    File::create(&file_name)
                        .await?
                        .write_all(include_bytes!("404.png"))
                        .await?;
                    e.insert(file_name.strip_prefix(&state.root_dir)?.to_path_buf());
                }
            }
            other => bail!("Unsupported emoji type: {other:?}"),
        };
    }
    let emoji = state
        .emojis
        .get(reaction)
        .expect("Failed to retrieve emoji reference");
    Ok(emoji)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ReactionStore {
    path: PathBuf,
    count: u64,
}
async fn process_message(state: &mut ArchivalState, message: &mut Message) -> Result<()> {
    let asset_path = &state.assets_dir;
    futures::future::join_all(
        message
            .attachments
            .iter()
            .map(move |attachment| async move {
                let filename = format!("{}_{}", attachment.id, attachment.filename);
                let file_path = asset_path.join(&filename);
                download_to_file(&attachment.url, &file_path)
                    .await
                    .context("Downloading attachment")?;
                Result::<(), Error>::Ok(())
            }),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<()>, _>>()?;

    ensure_user_avatar(state, &mut message.author)
        .await
        .context("fetching user avatar")?;

    let mut reactions = vec![];
    for reaction in &message.reactions {
        let path = ensure_emoji(state, &reaction.reaction_type)
            .await
            .with_context(|| format!("fetching emoji {}", reaction.reaction_type))?;
        reactions.push(ReactionStore {
            count: reaction.count,
            path: path.to_path_buf(),
        })
    }

    let emoji_regex = lazy_regex::regex!(r"<:.+?:([^:>]+)>");
    for emoji in emoji_regex
        .captures_iter(&message.content)
        .filter_map(|e| e.get(1).and_then(|e| e.as_str().parse::<u64>().ok()))
        .map(|id| ReactionType::Custom {
            animated: false,
            id: EmojiId::from(id),
            name: None,
        })
    {
        ensure_emoji(state, &emoji)
            .await
            .with_context(|| format!("fetching emoji {}", emoji))?;
    }

    // EMOJI_REGEX.find_iter(message.content)

    let mut json_string = if state.processed_count > 0 {
        ",\n"
    } else {
        "[\n"
    }
    .to_string();

    let mut json_obj = serde_json::to_value(&message)?;
    json_obj["reactions::processed"] = serde_json::to_value(reactions)?;
    json_string += &serde_json::to_string(&json_obj)?;
    state.file.write_all(json_string.as_bytes()).await?;

    state.processed_count += 1;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "MANAGE_MESSAGES",
    default_member_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "MANAGE_MESSAGES|READ_MESSAGE_HISTORY",
    guild_only
)]
async fn archive(ctx: Context<'_>) -> Result<()> {
    handle_archive(ctx).await.map_err(|err| {
        let err = err
            .chain()
            .rev()
            .enumerate()
            .map(|(i, e)| match i {
                0 => format!("Got an error: {e}"),
                _ => format!("While {e}"),
            })
            .collect::<Vec<_>>()
            .join("\n");
        anyhow!("{err}")
    })
}

async fn handle_archive(ctx: Context<'_>) -> Result<()> {
    let confirmed = confirm(ctx).await?;
    if !confirmed {
        ctx.say("Operation canceled").await?;
    }

    let response = ctx
        .send(|msg| msg.content("Archival in progress").ephemeral(false))
        .await?;

    let mut state = ArchivalState::create().await?;

    let channel = ctx.channel_id();
    let mut messages = channel.messages_iter(&ctx).boxed();
    let mut last = Instant::now();
    let mut last_count = 0;
    while let Some(message) = messages.next().await {
        let mut message = message?;
        if (last.elapsed().as_secs() >= 1 && state.processed_count > last_count)
            && (last.elapsed().as_secs() >= 2 || state.processed_count - last_count >= 10)
        {
            last = Instant::now();
            last_count = state.processed_count;
            response
                .edit(ctx, |msg| {
                    msg.content(format!(
                        "Messages archived: {}\nCurrently processing: {}",
                        state.processed_count,
                        message.link()
                    ))
                })
                .await?;
        }
        process_message(&mut state, &mut message)
            .await
            .with_context(|| format!("processing message {}", message.id))?;
    }

    state.finalize().await?;

    response
        .edit(ctx, |msg| msg.content("Archiving files"))
        .await?;

    let mut file = NamedTempFile::new()?;
    archive_directory(state.root_dir.path(), file.as_file_mut()).context("zipping files")?;
    let size = file.as_file().metadata()?.len();

    response
        .edit(ctx, |msg| msg.content("Uploading archive"))
        .await?;

    let filename = "archive.zip";

    let timeout = 60 * 15;

    let edit_prefix = format!(
        "Archival successful. Messages deletion will automatically be canceled <t:{}:R>",
        Timestamp::now().unix_timestamp() + timeout
    );

    // Less than 25 MB limit to be safe
    const SIZE_LIMIT: u64 = 1000 * 1000 * 24;
    let mut latest_message = if size < SIZE_LIMIT {
        let tokio_file = File::open(file.path()).await?;
        ctx.channel_id()
            .send_files(
                ctx.serenity_context(),
                [(&tokio_file, "archive.zip")],
                |msg| msg.content(edit_prefix),
            )
            .await
            .context("uploading archive to discord")?
    } else {
        let uploaded = upload_file(file.path(), filename.to_string())
            .await
            .context("uploading archive to file.io")?;
        response
            .edit(ctx, |msg| {
                msg.content(format!(
                    "{edit_prefix}\nDownload archive at {}\nFile will expire <t:{}:R>, or after {}",
                    uploaded.link,
                    uploaded.expires.unix_timestamp(),
                    pluralize("downloads", uploaded.max_downloads, true)
                ))
            })
            .await?;
        response.into_message().await?
    };

    latest_message
        .edit(ctx, |msg| {
            msg.components(|c| {
                c.create_action_row(|row| {
                    row.create_button(|btn| {
                        btn.label("Wipe archived messages")
                            .style(ButtonStyle::Danger)
                            .custom_id("delete")
                    })
                    .create_button(|btn| {
                        btn.label("Cancel")
                            .style(ButtonStyle::Primary)
                            .custom_id("cancel")
                    })
                })
            })
        })
        .await?;

    let mut builder = ComponentInteractionCollectorBuilder::new(ctx)
        .message_id(latest_message.id)
        .author_id(ctx.author().id)
        .collect_limit(1)
        .timeout(Duration::from_secs(timeout as u64))
        .build();

    let delete = match builder.next().await {
        None => false,
        Some(interaction) => {
            interaction
                .create_interaction_response(ctx.serenity_context(), |r| {
                    r.kind(InteractionResponseType::DeferredUpdateMessage)
                })
                .await?;
            interaction.data.custom_id == "delete"
        }
    };

    if delete {
        bail!("NOT IMPLEMENTED!");
    } else {
        latest_message
            .edit(ctx, |msg| {
                msg.components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|btn| {
                            btn.label("Wiping canceled")
                                .disabled(true)
                                .custom_id("canceled")
                        })
                    })
                })
            })
            .await?;
    }

    file.close()?;

    Ok(())
}

#[poise::command(prefix_command)]
async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![register(), archive()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("rw!".to_string()),
                ..Default::default()
            },
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT)
        .setup(|_ctx, _ready, _framework| Box::pin(async move { Ok(Data {}) }));

    framework.run().await.unwrap();
}
