use anyhow::Result;
use archival::archive_command;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};
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
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![register(), archive()],
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
