use crate::into_edit::IntoEdit;
use poise::serenity_prelude::{CreateButton, EditMessage, Message};

pub async fn clear_components<T: Send + Sync>(
    ctx: poise::Context<'_, T, anyhow::Error>,
    message: &mut Message,
) -> anyhow::Result<()> {
    message
        .edit(ctx, EditMessage::new().components(vec![]))
        .await?;
    Ok(())
}

pub async fn set_dummy_text_component<T: Send + Sync>(
    ctx: poise::Context<'_, T, anyhow::Error>,
    message: &mut Message,
    text: impl Into<String>,
) -> anyhow::Result<()> {
    message
        .edit(
            ctx,
            CreateButton::new("-")
                .disabled(true)
                .label(text.into())
                .into_edit(),
        )
        .await?;
    Ok(())
}
