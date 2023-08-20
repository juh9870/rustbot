use poise::serenity_prelude::Message;

pub async fn clear_components<T: Sync>(
    ctx: poise::Context<'_, T, anyhow::Error>,
    message: &mut Message,
) -> anyhow::Result<()> {
    message.edit(ctx, |msg| msg.components(|c| c)).await?;
    Ok(())
}

pub async fn set_dummy_text_component<T: Sync>(
    ctx: poise::Context<'_, T, anyhow::Error>,
    message: &mut Message,
    text: impl Into<String>,
) -> anyhow::Result<()> {
    message
        .edit(ctx, |msg| {
            msg.components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|btn| btn.label(text.into()).disabled(true).custom_id("-"))
                })
            })
        })
        .await?;
    Ok(())
}
