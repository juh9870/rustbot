use camino::Utf8Path;
use chrono::{DateTime, Utc};
use poise::CreateReply;
use poise::serenity_prelude::{
    CacheHttp, ChannelId, CreateAllowedMentions, CreateAutocompleteResponse, CreateMessage,
    MessageBuilder, UserId,
};
use rootcause::ReportRef;
use rootcause::prelude::ResultExt;
use rootcause::report_collection::ReportCollection;
use sqlx::{Connection, SqliteConnection, migrate, query};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};
use utils::error_handle::transform_error;
use utils::poise_data::{PoiseContext, PoiseResources};

pub mod time;

/// Creates a reminder for bot to ping you with after a delay
#[poise::command(rename = "remind", slash_command, prefix_command, guild_only)]
pub async fn remind_command(
    ctx: PoiseContext<'_>,
    #[description = "Time after which to remind. Examples: `5 minutes`, `in an hour`, `1d 16h 35m`"]
    #[autocomplete = "parse_timestamp"]
    #[string]
    timestamp: ChronoTimestampArg,
    #[description = "Reminder message"]
    #[string]
    message: LimitedMessageArg,
) -> rootcause::Result<()> {
    remind(ctx, timestamp, message.0)
        .await
        .map_err(transform_error)
}

#[derive(Debug)]
struct ReminderResource {
    conn: SqliteConnection,
}
pub async fn init(resources: &mut PoiseResources, data_path: &Utf8Path) -> rootcause::Result<()> {
    let db_url = &format!(
        "sqlite://{}?mode=rwc",
        data_path.join("reminders.db").as_str()
    );

    let mut conn = SqliteConnection::connect(db_url)
        .await
        .context("Failed to connect to reminders database")?;

    migrate!("../migrations").run(&mut conn).await?;

    resources.init_resource::<ReminderResource>(ReminderResource { conn });

    Ok(())
}

pub async fn run_loop(
    scheduler: &mut JobScheduler,
    resources: Arc<PoiseResources>,
    client: Arc<impl CacheHttp + 'static>,
) -> rootcause::Result<()> {
    scheduler
        .add(
            Job::new_repeated_async(Duration::from_secs(5), move |_, _| {
                let cl = client.clone();
                let res = resources.clone();
                Box::pin(async move {
                    if let Err(err) = process_reminders(&res, cl)
                        .await
                        .context("Failed to process reminders loop")
                    {
                        error!("{:?}", err)
                    }
                })
            })
            .context("Failed to initialize job")?,
        )
        .await
        .context("Failed to add job")?;

    Ok(())
}

struct ChronoTimestampArg(DateTime<Utc>);

impl FromStr for ChronoTimestampArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        time::parse_ts(s)
            .map(|t| Self(t.0))
            .map_err(|e| e.to_string())
    }
}

struct LimitedMessageArg(String);

impl FromStr for LimitedMessageArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > 1000 {
            return Err(format!("Message too long - {}/1000", s.len()));
        }
        Ok(Self(s.to_string()))
    }
}

async fn parse_timestamp(_: PoiseContext<'_>, text: &str) -> CreateAutocompleteResponse {
    let mut opts = CreateAutocompleteResponse::new();
    let text = text.trim();
    if text.is_empty() {
        opts = opts.add_string_choice("Input a duration (for example: in 5 minutes)", "");
        return opts;
    }
    match time::parse_ts(text) {
        Ok((date, delta)) => {
            let date_str = time::format_date(date);
            let duration = time::format_duration(delta);
            opts = opts.add_string_choice(date_str, duration.clone());
            opts = opts.add_string_choice(duration.clone(), duration.clone());
        }
        Err(err) => {
            let mut txt = format!("Failed to parse duration - {}", err);
            if let Some(index) = txt.find(", supported units") {
                txt.truncate(index);
            }
            if txt.len() > 100 {
                txt.truncate(99);
                txt.push('…');
            }
            opts = opts.add_string_choice(txt, text);
        }
    }

    opts
}

async fn remind(
    ctx: PoiseContext<'_>,
    timestamp: ChronoTimestampArg,
    message: String,
) -> rootcause::Result<()> {
    let mut res = ctx.data().get_resource::<ReminderResource>().await?;

    let author_id = ctx.author().id.get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let rem_unix_ts = timestamp.0.timestamp();
    query!(
        r#"
    INSERT INTO reminders (message, author_id, channel_id, reminder_unix_ts) VALUES (?, ?, ?, ?);
    "#,
        message,
        author_id,
        channel_id,
        rem_unix_ts,
    )
    .execute(&mut res.conn)
    .await
    .context("Failed to record the reminder")?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Reminder set for <t:{}:s> (<t:{}:R>) - {}",
                rem_unix_ts, rem_unix_ts, message
            ))
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::new()),
    )
    .await?;

    Ok(())
}

async fn process_reminders(
    resources: &PoiseResources,
    client: impl CacheHttp,
) -> rootcause::Result<()> {
    info!("Processing reminders loop");
    let rems = Reminder::get_pending(resources).await?;

    let rems_count = rems.len();
    let mut errors = ReportCollection::new();
    for rem in rems {
        match rem.process(resources, &client).await {
            Ok(_) => {}
            Err(err) => {
                errors.push(err.into_cloneable());
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let errors_count = errors.len();
    Err(errors
        .context(format!(
            "Processed {}/{} reminders successfully",
            rems_count - errors_count,
            rems_count
        ))
        .into_dynamic())
}

struct Reminder {
    id: i64,
    author: UserId,
    channel: ChannelId,
    text: String,
}

impl Reminder {
    async fn process(
        self,
        resources: &PoiseResources,
        client: impl CacheHttp,
    ) -> rootcause::Result<()> {
        let content = MessageBuilder::new()
            .mention(&self.author)
            .push_line(" Reminder:")
            .push_quote_safe(self.text)
            .build();

        let msg = CreateMessage::new()
            .content(content)
            .allowed_mentions(CreateAllowedMentions::new().users([self.author]));

        if let Err(err) = self
            .channel
            .send_message(client, msg)
            .await
            .context("Failed to send the reminder")
        {
            if is_error_discardable(err.as_ref()) {
                warn!(?err, "Discarded bad reminder")
            } else {
                return Err(err.into_dynamic());
            }
        }

        Self::remove_from_db(self.id, resources).await
    }

    async fn remove_from_db(id: i64, resources: &PoiseResources) -> rootcause::Result<()> {
        let mut res = resources.get_resource::<ReminderResource>().await?;

        query!("DELETE FROM reminders WHERE id = ?", id)
            .execute(&mut res.conn)
            .await
            .context("Failed to remove sent reminder from the DB")?;
        Ok(())
    }

    async fn get_pending(resources: &PoiseResources) -> rootcause::Result<Vec<Self>> {
        let mut res = resources.get_resource::<ReminderResource>().await?;

        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        let reminders = query!(
            r#"
        SELECT id, message, author_id, channel_id, reply_to from reminders
        WHERE reminder_unix_ts <= ?
        "#,
            now_unix
        )
        .fetch_all(&mut res.conn)
        .await
        .context("Failed to query reminders from the DB")?;

        Ok(reminders
            .into_iter()
            .map(|r| Self {
                id: r.id,
                author: UserId::new(r.author_id as u64),
                channel: ChannelId::new(r.channel_id as u64),
                text: r.message,
            })
            .collect())
    }
}

fn is_error_discardable<M: ?Sized, C>(err: ReportRef<M, C>) -> bool {
    if err
        .format_current_context_unhooked()
        .to_string()
        .contains("Unknown Channel")
    {
        return true;
    }
    for c in err.children() {
        if is_error_discardable(c) {
            return true;
        }
    }

    false
}
