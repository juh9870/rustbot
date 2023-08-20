use anyhow::Result;
use futures::StreamExt;
use poise::serenity_prelude::{
    ButtonStyle, ComponentInteractionCollectorBuilder, InteractionResponseType, Message,
};

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

pub async fn confirm_buttons<T: Sync>(
    ctx: poise::Context<'_, T, anyhow::Error>,
    message: &mut Message,
    options: BtnConfirmOptions,
) -> Result<ConfirmationResult> {
    message
        .edit(ctx, |msg| {
            msg.components(|c| {
                c.create_action_row(|row| {
                    row.create_button(|btn| {
                        btn.label(options.confirm_text)
                            .style(options.confirm_style)
                            .custom_id("confirm")
                    })
                    .create_button(|btn| {
                        btn.label(options.cancel_text)
                            .style(options.cancel_style)
                            .custom_id("cancel")
                    })
                })
            })
        })
        .await?;

    let mut builder = ComponentInteractionCollectorBuilder::new(ctx)
        .message_id(message.id)
        .author_id(ctx.author().id)
        .collect_limit(1)
        .timeout(options.timeout)
        .build();

    let confirmed = match builder.next().await {
        None => false,
        Some(interaction) => {
            interaction
                .create_interaction_response(ctx.serenity_context(), |r| {
                    r.kind(InteractionResponseType::DeferredUpdateMessage)
                })
                .await?;
            interaction.data.custom_id == "confirm"
        }
    };
    Ok(confirmed.into())
}
