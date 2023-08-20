use anyhow::Result;
use archival;
use archival::archive_command;
use poise::serenity_prelude::GatewayIntents;
use poise::PrefixFrameworkOptions;

struct Data {}

type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

archive_command!(archive, Data);

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
