use anyhow::Result;
use futures::StreamExt;
use poise::serenity_prelude::{
    ButtonStyle, ComponentInteractionCollector, CreateButton, CreateInteractionResponse, Message,
};

use crate::into_edit::IntoEdit;
use std::time::Duration;

pub struct BtnConfirmOptions {
    pub confirm_text: String,
    pub confirm_style: ButtonStyle,
    pub cancel_text: String,
    pub cancel_style: ButtonStyle,
    pub timeout: Duration,
}

#[must_use]
#[derive(Debug, Copy, Clone)]
pub enum ConfirmationResult {
    Confirmed,
    Canceled,
}

impl ConfirmationResult {
    pub fn bool(&self) -> bool {
        match self {
            ConfirmationResult::Confirmed => true,
            ConfirmationResult::Canceled => false,
        }
    }
}

impl From<ConfirmationResult> for bool {
    fn from(value: ConfirmationResult) -> Self {
        value.bool()
    }
}

impl From<&ConfirmationResult> for bool {
    fn from(value: &ConfirmationResult) -> Self {
        value.bool()
    }
}

impl From<bool> for ConfirmationResult {
    fn from(value: bool) -> Self {
        if value {
            ConfirmationResult::Confirmed
        } else {
            ConfirmationResult::Canceled
        }
    }
}

pub async fn confirm_buttons<T: Send + Sync>(
    ctx: poise::Context<'_, T, anyhow::Error>,
    message: &mut Message,
    options: BtnConfirmOptions,
) -> Result<ConfirmationResult> {
    let buttons = vec![
        CreateButton::new("confirm")
            .label(options.confirm_text)
            .style(options.confirm_style),
        CreateButton::new("cancel")
            .label(options.cancel_text)
            .style(options.cancel_style),
    ]
    .into_edit();
    message.edit(ctx, buttons).await?;

    let mut interactions = ComponentInteractionCollector::new(ctx)
        .message_id(message.id)
        .author_id(ctx.author().id)
        .timeout(options.timeout)
        .stream()
        .take(1);

    let confirmed = match interactions.next().await {
        None => false,
        Some(interaction) => {
            interaction
                .create_response(
                    ctx.serenity_context(),
                    CreateInteractionResponse::Acknowledge,
                )
                .await?;
            interaction.data.custom_id == "confirm"
        }
    };
    Ok(confirmed.into())
}
