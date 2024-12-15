use anyhow::Result;
use archival::archive_command;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};
use poise::PrefixFrameworkOptions;

struct Data {}

type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

archive_command!(archive, Data);

#[poise::command(prefix_command, owners_only, hide_in_help)]
async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

/// Help command
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<()> {
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
Type /help command for more info on a command.",
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![register(), archive(), help()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("dh!".to_string()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|_ctx, _ready, _framework| {
            // poise::builtins::register_globally(ctx, &framework.options().commands).await?;
            Box::pin(async move { Ok(Data {}) })
        })
        .build();

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    println!("Bot started");
    client.unwrap().start().await.unwrap();
}
