use camino::{Utf8Path, Utf8PathBuf};
use chrono::SecondsFormat;
use rootcause::prelude::ResultExt;
use serenity::all::{
    ChannelId, EventHandler, GuildId, Http, Message, MessageId, Ready, Timestamp, UserId,
};
use serenity::async_trait;
use serenity::client::{ClientBuilder, Context};
use serenity::futures::StreamExt;
use serenity::prelude::GatewayIntents;
use sqlx::__rt::timeout;
use sqlx::{Connection, SqliteConnection, migrate, query};
use std::ops::DerefMut;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{Instrument, error, info, instrument, warn};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use utils::messages_iter::{MessageRangeInChannel, Resume, ResumeResult, smart_messages_iter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(rootcause_tracing::RootcauseLayer)
        .with(
            tracing_subscriber::filter::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().pretty().boxed())
        .init();

    if let Err(err) = main_inner().in_current_span().await {
        error!(?err, "Failed to run the bot");
        exit(1)
    }
}

#[instrument]
async fn main_inner() -> rootcause::Result<()> {
    let token = std::env::var("DISCORD_TOKEN").context("missing DISCORD_TOKEN")?;
    let data_path =
        Utf8PathBuf::from_str(&std::env::var("DATA_PATH").context("missing DATA_PATH")?)
            .context("Bad DATA_PATH")?;
    let channel = ChannelId::from_str(&std::env::var("CHANNEL_ID").context("missing CHANNEL_ID")?)
        .context("bad CHANNEL_ID")?;
    let yeet = &std::env::var("DO_YEET").unwrap_or_default() == "1";
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let fr = FileResumer::open(channel, &data_path)
        .await
        .context("Failed to initialize file resumer")?;

    let mut client = ClientBuilder::new(token, intents)
        .event_handler(Handler { fr, yeet })
        .await
        .context("Failed to initialize poise client")?;

    client.start().await.context("Failed to start the bot")?;
    Ok(())
}

struct Handler {
    fr: FileResumer,
    yeet: bool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        info!("Bot ready");

        if self.yeet {
            info!("Yeeting");
            if let Err(e) = yeet_messages(ctx, self.fr.clone()).await {
                error!(err=?e, "Process failed");
                exit(1)
            }
        } else {
            if let Err(e) = find_messages(ctx, self.fr.clone()).await {
                error!(err=?e, "Process failed");
                exit(1)
            }
        }
        exit(0)
    }
}

async fn find_messages(ctx: Context, mut fr: FileResumer) -> rootcause::Result<()> {
    let user = UserId::from_str(&std::env::var("USER_ID").context("missing USER_ID")?)
        .context("bad USER_ID")?;

    let mut iter = smart_messages_iter(
        ctx.http.clone(),
        MessageRangeInChannel::new_full(fr.channel_id),
        fr.clone(),
    )
    .boxed();

    while let Some(item) = iter.next().await {
        let item = item.context("Failed to fetch message")?;
        if item.author.id != user {
            continue;
        }

        let guild = item.guild_id.unwrap_or(GuildId::default());

        fr.record_message(&item).await.context_with(|| {
            format!(
                "Failed to record message {} in channel {} in guild {}",
                item.id, item.channel_id, guild
            )
        })?;
    }

    Ok(())
}

async fn yeet_messages(ctx: Context, fr: FileResumer) -> rootcause::Result<()> {
    let timeout_sec = u64::from_str(
        &std::env::var("DELETE_TIMEOUT_SEC").context("DELETE_TIMEOUT_SEC is missing")?,
    )
    .context("bad DELETE_TIMEOUT_SEC value")?;
    let batch_size = u64::from_str(&std::env::var("BATCH_SIZE").context("BATCH_SIZE is missing")?)
        .context("bad BATCH_SIZE value")?;
    let skip_failed_deletions = &std::env::var("SKIP_FAILED_DELETIONS").unwrap_or_default() == "1";

    let mut conn = fr.conn.lock().await;

    let row = query!("SELECT count(*) AS total FROM deletion_priority")
        .fetch_one(conn.deref_mut())
        .await
        .context("Failed to fetch deletion queue count")?;

    let total = row.total;
    let mut yeeted: i64 = 0;
    let mut skipped: i64 = 0;

    let mut discord_delete_timing = Timing::default();
    let mut db_delete_timing = Timing::default();

    let start = Instant::now();
    info!("Starting deletion");
    loop {
        let batch = query!(
            "SELECT message_id, channel_id FROM deletion_priority ORDER BY priority DESC, message_id LIMIT ?",
            batch_size as i64
        )
            .fetch_all(conn.deref_mut())
            .await
            .context("Failed to fetch deletion batch")?;

        info!("Fetched a batch");

        if batch.is_empty() {
            info!("No more queued messages, bailing");
            return Ok(());
        }
        for msg in batch {
            let channel = ChannelId::new(msg.channel_id as u64);
            let msg_id = MessageId::new(msg.message_id as u64);
            let deletion_start = Instant::now();

            if let Err(e) =
                try_delete_message(&ctx.http, channel, msg_id, Duration::new(timeout_sec, 0)).await
            {
                if e.to_string().contains("Unknown Message") {
                    // ignore
                } else {
                    warn!("{}", e);
                    discord_delete_timing.record(deletion_start.elapsed());
                    if skip_failed_deletions {
                        skipped += 1;
                        continue;
                    }
                    return Err(e);
                }
            }

            discord_delete_timing.record(deletion_start.elapsed());

            let db_deletion_start = Instant::now();
            query!(
                r#"
                UPDATE messages SET deleted = 1 where message_id = ?;
                DELETE FROM deletion_priority where message_id = ?;
                "#,
                msg.message_id,
                msg.message_id
            )
            .execute(conn.deref_mut())
            .await
            .context_with(|| format!("Failed to record deletion of message {:?}", msg_id))?;
            db_delete_timing.record(db_deletion_start.elapsed());

            yeeted += 1;
            info!(
                "{}; discord: {:.2}; db: {:.2}",
                progress_bar(start.elapsed(), yeeted, skipped, total),
                discord_delete_timing.avg(),
                db_delete_timing.avg()
            )
        }
    }
}

async fn try_delete_message(
    http: impl AsRef<Http>,
    channel: ChannelId,
    msg: MessageId,
    timeout_dur: Duration,
) -> rootcause::Result<()> {
    timeout(timeout_dur, channel.delete_message(http, msg))
        .await
        .context("Discord message deletion is stuck")?
        .context_with(|| format!("Failed to delete message {:?}", msg))?;

    Ok(())
}

#[derive(Default)]
struct Timing {
    count: u64,
    total: Duration,
}
impl Timing {
    fn record(&mut self, elapsed: Duration) {
        self.count += 1;
        self.total += elapsed;
    }

    fn avg(&self) -> f64 {
        self.total.as_secs_f64() / self.count as f64
    }
}

fn progress_bar(elapsed: Duration, done: i64, skipped: i64, total: i64) -> String {
    let progress = (done as f64) / (total as f64);

    let items_per_second = done as f64 / elapsed.as_secs_f64();
    let mut remaining_seconds = (total.saturating_sub(done) as f64 / items_per_second) as u64;
    let mut bulk = String::new();
    if remaining_seconds > 3600 {
        bulk.push_str(&format!("{}h ", remaining_seconds / 3600));
        remaining_seconds %= 3600;
    }
    if remaining_seconds > 60 {
        bulk.push_str(&format!("{}m ", remaining_seconds / 60));
        remaining_seconds %= 60;
    }
    bulk.push_str(&format!("{}s ", remaining_seconds));

    format!(
        "{}+{}/{} ({}%) ({:.2} m/s, {} est)",
        done,
        skipped,
        total,
        (progress * 100.0) as u32,
        items_per_second,
        bulk.trim(),
    )
}
#[derive(Debug, Clone)]
struct FileResumer {
    conn: Arc<Mutex<SqliteConnection>>,
    channel_id: ChannelId,
}

impl FileResumer {
    async fn open(cid: ChannelId, data_path: &Utf8Path) -> rootcause::Result<Self> {
        let db_url = &format!(
            "sqlite://{}?mode=rwc",
            data_path.join("messages.db").as_str()
        );

        let mut conn = SqliteConnection::connect(db_url)
            .await
            .context("Failed to connect to messages database")?;

        migrate!("./migrations").run(&mut conn).await?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            channel_id: cid,
        })
    }

    async fn record_message(&mut self, message: &Message) -> rootcause::Result<()> {
        let mut conn = self.conn.lock().await;
        let guild = message.guild_id.unwrap_or_default();
        let timestamp = message.timestamp;
        query!(
            "INSERT INTO messages (message_id, channel_id, guild_id, timestamp, content, attachments_count) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT DO NOTHING",
            message.id.get() as i64,
            message.channel_id.get() as i64,
            guild.get() as i64,
            timestamp.timestamp(),
            message.content,
            message.attachments.len() as i64,
        ).execute(conn.deref_mut()).await?;
        Ok(())
    }
}

impl Resume for FileResumer {
    async fn record(&mut self, before: (MessageId, Timestamp)) -> rootcause::Result<()> {
        let ts_display = before.1.to_rfc3339_opts(SecondsFormat::Secs, true);
        info!(message_timestamp = ts_display, message_id = ?before.0, "Recording a batch");
        let cid = self.channel_id.get() as i64;
        let prog = before.0.get() as i64;
        let prog_ts = before.1.timestamp();
        let mut conn = self.conn.lock().await;
        query!(
            "INSERT INTO progress (channel_id, progress, progress_ts) VALUES (?, ?, ?) ON CONFLICT DO UPDATE SET progress = EXCLUDED.progress, progress_ts = EXCLUDED.progress_ts",
            cid, prog, prog_ts
        ).execute(conn.deref_mut()).await?;
        Ok(())
    }

    async fn mark_done(&mut self) -> rootcause::Result<()> {
        let cid = self.channel_id.get() as i64;
        let mut conn = self.conn.lock().await;
        query!(
            "INSERT INTO progress (channel_id, done) VALUES (?, 1) ON CONFLICT DO UPDATE SET done = EXCLUDED.done",
            cid,
        ).execute(conn.deref_mut()).await?;
        Ok(())
    }

    async fn resume(&mut self) -> rootcause::Result<ResumeResult> {
        let cid = self.channel_id.get() as i64;
        let mut conn = self.conn.lock().await;
        let row = query!(
            "SELECT done, progress, progress_ts FROM progress WHERE channel_id = ?",
            cid
        )
        .fetch_optional(conn.deref_mut())
        .await?;
        let Some(row) = row else {
            return Ok(ResumeResult::NotRecorded);
        };
        if row.done.is_some_and(|d| d == 1) {
            return Ok(ResumeResult::Done);
        }
        if let Some(progress) = row.progress
            && let Some(ts) = row.progress_ts
        {
            return Ok(ResumeResult::Recorded((
                MessageId::new(progress as u64),
                Timestamp::from_unix_timestamp(ts)?,
            )));
        }
        Ok(ResumeResult::NotRecorded)
    }
}
