use anyhow::{anyhow, bail, Context, Result};
use futures::Stream;
use futures::StreamExt;
use poise::serenity_prelude::{
    ChannelId, EmojiId, Message, ReactionType, StickerId, StickerItem, Timestamp, User, UserId,
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::{tempdir, NamedTempFile, TempDir};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use twemoji_assets::png::PngTwemojiAsset;
use utils::web_files::download_to_file;
use utils::zip::archive_directory;

#[derive(Debug)]
struct ArchivalState {
    time_range: Option<Range<Timestamp>>,
    root_dir: TempDir,
    assets_dir: PathBuf,
    file: File,
    processed_count: usize,
    avatars: FxHashMap<UserId, PathBuf>,
    emojis: FxHashMap<ReactionType, PathBuf>,
    stickers: FxHashMap<StickerId, PathBuf>,
}

impl ArchivalState {
    async fn create() -> Result<Self> {
        let dir = tempdir()?;
        let file = File::create(dir.path().join("messages.jsonp")).await?;
        File::create(dir.path().join("archive.html"))
            .await?
            .write_all(include_bytes!("../archive_viewer/dist/archive.html"))
            .await?;
        let assets_dir = dir.path().join("assets");
        tokio::fs::create_dir(&assets_dir).await?;
        Ok(ArchivalState {
            time_range: None,
            root_dir: dir,
            assets_dir,
            file,
            processed_count: 0,
            avatars: Default::default(),
            emojis: Default::default(),
            stickers: Default::default(),
        })
    }

    async fn finalize(&mut self) -> Result<()> {
        self.file.write_all("\n])".as_bytes()).await?;
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

async fn ensure_sticker<'a>(
    state: &'a mut ArchivalState,
    sticker: &StickerItem,
) -> Result<&'a Path> {
    if let std::collections::hash_map::Entry::Vacant(e) = state.stickers.entry(sticker.id) {
        let image_url = sticker
            .image_url()
            .ok_or_else(|| anyhow!("Sticker image URL missing"))?;
        println!("Sticker: {image_url}");
        let extension = get_extension_from_url(&image_url)?;
        let file_path = state.assets_dir.join(format!("{}.{extension}", sticker.id));
        download_to_file(&image_url, &file_path).await?;
        e.insert(file_path.strip_prefix(&state.root_dir)?.to_path_buf());
    }

    Ok(state
        .stickers
        .get(&sticker.id)
        .expect("Failed to retrieve sticker reference"))
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
                let asset = PngTwemojiAsset::from_emoji(emoji).or_else(|| {
                    PngTwemojiAsset::from_emoji(
                        &emoji
                            .chars()
                            .next()
                            .expect("Empty reaction string")
                            .to_string(),
                    )
                });
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

async fn process_message<Data>(
    ctx: poise::Context<'_, Data, anyhow::Error>,
    state: &mut ArchivalState,
    message: &mut Message,
) -> Result<()> {
    let asset_path = &state.assets_dir;
    let root_dir_path = state.root_dir.path();
    futures::future::join_all(
        message
            .attachments
            .iter_mut()
            .map(move |attachment| async move {
                let filename = format!("{}_{}", attachment.id, attachment.filename);
                let file_path = asset_path.join(&filename);
                download_to_file(&attachment.url, &file_path)
                    .await
                    .context("Downloading attachment")?;
                attachment.url = file_path
                    .strip_prefix(root_dir_path)?
                    .to_str()
                    .ok_or_else(|| anyhow!("Bad file path"))?
                    .to_string();
                Result::<()>::Ok(())
            }),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<()>, _>>()
    .context("downloading attachments")?;

    let mut stickers = vec![];
    for sticker in &message.sticker_items {
        let path = ensure_sticker(state, sticker)
            .await
            .with_context(|| format!("Fetching sticker {}", sticker.id))?;
        stickers.push((sticker.id, path.to_owned()));
    }
    let stickers = stickers.into_iter().collect::<FxHashMap<_, _>>();

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

    let emoji_regex = lazy_regex::regex!(r"<:\w+:(\d{18})>");
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

    let mut channel_names = vec![];
    let channel_regex = lazy_regex::regex!(r"<#(\d{18})>");
    let channel_ids = channel_regex
        .captures_iter(&message.content)
        .filter_map(|e| e.get(1).and_then(|e| e.as_str().parse::<u64>().ok()))
        .map(ChannelId::from)
        .collect::<Vec<_>>();
    for channel_id in &channel_ids {
        channel_names.push(channel_id.name(ctx));
    }

    let channel_names: FxHashMap<ChannelId, String> = futures::future::join_all(channel_names)
        .await
        .into_iter()
        .zip(channel_ids)
        .filter_map(|(name, id)| name.map(|name| (id, name)))
        .collect();

    let mut roles = vec![];
    for role in &message.mention_roles {
        let role = role.to_role_cached(ctx);
        if let Some(role) = role {
            roles.push(role);
        }
    }

    // EMOJI_REGEX.find_iter(message.content)

    let mut json_string = if state.processed_count > 0 {
        ",\n"
    } else {
        "jsonp_parse([\n"
    }
    .to_string();

    let mut json_obj = serde_json::to_value(&message).context("serializing main message data")?;
    json_obj["reactions::processed"] =
        serde_json::to_value(reactions).context("serializing reactions")?;
    json_obj["mention_roles::processed"] =
        serde_json::to_value(roles).context("serializing role mentions")?;
    json_obj["mention_channels::processed"] =
        serde_json::to_value(channel_names).context("serializing channel mentions")?;
    json_obj["stickers::processed"] =
        serde_json::to_value(stickers).context("serializing used stickers")?;
    json_string += &serde_json::to_string(&json_obj).context("stringifying json")?;
    state
        .file
        .write_all(json_string.as_bytes())
        .await
        .context("writing to a file")?;

    state.processed_count += 1;
    match &mut state.time_range {
        None => {
            state.time_range = Some(message.timestamp..message.timestamp);
        }
        Some(range) => {
            range.start = range.start.min(message.timestamp);
            range.end = range.end.max(message.timestamp);
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct ArchiveData {
    pub file: NamedTempFile,
    pub time_range: Range<Timestamp>,
}

pub async fn archive_messages<
    Data,
    Messages: Stream<Item = Result<Message>> + Send,
    Reporter: Fn(String) -> ReportResult,
    ReportResult: Future<Output = Result<()>>,
>(
    ctx: poise::Context<'_, Data, anyhow::Error>,
    messages: Messages,
    report: Reporter,
) -> Result<ArchiveData> {
    let mut state = ArchivalState::create().await?;

    let mut messages = messages.boxed();
    let mut last = Instant::now();
    let mut last_count = 0;
    while let Some(message) = messages.next().await {
        let mut message = message?;
        if (last.elapsed().as_secs() >= 1 && state.processed_count > last_count)
            && (last.elapsed().as_secs() >= 2 || state.processed_count - last_count >= 10)
        {
            last = Instant::now();
            last_count = state.processed_count;
            report(format!(
                "Messages archived: {}\nCurrently processing: {}",
                state.processed_count,
                message.link()
            ))
            .await?;
        }
        process_message(ctx, &mut state, &mut message)
            .await
            .with_context(|| format!("processing message {}", message.link()))?;
    }

    state.finalize().await?;

    let time_range = state
        .time_range
        .unwrap_or_else(|| Timestamp::now()..Timestamp::now());

    report("Archiving files".to_string()).await?;
    let mut file = NamedTempFile::new().context("creating archive file")?;
    archive_directory(state.root_dir.path(), file.as_file_mut()).context("zipping files")?;

    Ok(ArchiveData { file, time_range })
}
