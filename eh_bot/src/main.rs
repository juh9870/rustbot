use archival::archive_command;
use camino::Utf8PathBuf;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};
use poise::PrefixFrameworkOptions;
use reminders::remind_command;
use rootcause::prelude::ResultExt;
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;
use tokio_cron_scheduler::JobScheduler;
use tracing::{error, info, instrument, Instrument};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;
use utils::poise_data::{PoiseContext, PoiseResources};

#[poise::command(prefix_command, owners_only, hide_in_help)]
async fn register(ctx: PoiseContext<'_>) -> rootcause::Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

/// Help command
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: PoiseContext<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> rootcause::Result<()> {
    let config = help::HelpConfiguration {
        extra_text_at_bottom: "\
Type /help command for more info on a command.",
        ..Default::default()
    };
    help::help(ctx, command.as_deref(), config).await?;

    Ok(())
}

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
        error!(?err, "Failed to run the bot")
    }
}

#[instrument]
async fn main_inner() -> rootcause::Result<()> {
    let token = std::env::var("DISCORD_TOKEN").context("missing DISCORD_TOKEN")?;
    let data_path = std::env::var("DATA_PATH").context("missing DATA_PATH")?;
    let data_path =
        Utf8PathBuf::from_str(&data_path).context("DATA_PATH is not a valid UTF-8 path")?;
    if !data_path.exists() {
        fs_err::create_dir_all(&data_path).context("Failed to create data directory")?;
    }
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut resources = PoiseResources::default();
    reminders::init(&mut resources, &data_path)
        .await
        .context("Failed to initialize reminders")?;
    let resources = Arc::new(resources);

    let mut scheduler = JobScheduler::new()
        .await
        .context("Failed to initialize JobScheduler")?;

    let framework_res = resources.clone();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![register(), archive_command(), remind_command(), help()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(Cow::Borrowed("dh!")),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |_ctx, _ready, _framework| Box::pin(async move { Ok(framework_res) }))
        .build();

    let mut client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .context("Failed to initialize poise client")?;

    reminders::run_loop(&mut scheduler, resources.clone(), client.http.clone())
        .await
        .context("Failed to run reminders loop")?;

    scheduler
        .start()
        .await
        .context("Failed to run jobs scheduler")?;

    info!("Bot started");
    client.start().await.context("Failed to start the bot")?;
    Ok(())
}
