use anyhow::{anyhow, bail, Context, Result};
use futures::Stream;
use futures::StreamExt;
use poise::serenity_prelude::{EmojiId, Message, ReactionType, Timestamp, User, UserId};
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
}

impl ArchivalState {
    async fn create() -> Result<Self> {
        let dir = tempdir()?;
        let file = File::create(dir.path().join("messages.json")).await?;
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
                Result::<()>::Ok(())
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
    Messages: Stream<Item = Result<Message>> + Send,
    Reporter: Fn(String) -> ReportResult,
    ReportResult: Future<Output = Result<()>>,
>(
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
        process_message(&mut state, &mut message)
            .await
            .with_context(|| format!("processing message {}", message.id))?;
    }

    state.finalize().await?;

    let time_range = state
        .time_range
        .unwrap_or_else(|| Timestamp::now()..Timestamp::now());

    report("Archiving files".to_string()).await?;
    let mut file = NamedTempFile::new()?;
    archive_directory(state.root_dir.path(), file.as_file_mut()).context("zipping files")?;

    Ok(ArchiveData { file, time_range })
}
