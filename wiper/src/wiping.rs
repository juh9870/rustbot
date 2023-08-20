use anyhow::Result;
use chrono::Days;
use futures::{Stream, StreamExt};
use poise::serenity_prelude::{Message, Timestamp};
use std::future::Future;
use std::time::Duration;
use utils::reporter::{CountingReporter, Reporter, SimpleReporter};

const BATCH_SIZE: usize = 16;

pub async fn wipe_messages<
    Messages: Stream<Item = Result<Message>> + Send,
    Reporter: Fn(String, bool) -> ReportResult,
    ReportResult: Future<Output = Result<()>>,
    Data: Sync,
>(
    ctx: poise::Context<'_, Data, anyhow::Error>,
    messages: Messages,
    report: Reporter,
) -> Result<()> {
    let two_weeks_ago = Timestamp::from(
        Timestamp::now()
            .checked_sub_days(Days::new(0))
            .expect("Invalid system clock time"),
    );

    let mut reporter = SimpleReporter::new(Duration::from_secs(1), report);

    reporter.report("Fetching messages".to_string()).await?;

    let mut messages = messages.boxed();

    let mut initial_bulk = vec![];
    let mut next_message = None;
    while let Some(message) = messages.next().await {
        let message = message?;
        if message.timestamp > two_weeks_ago {
            initial_bulk.push(message);
        } else {
            next_message = Some(message);
            break;
        }
    }

    reporter
        .report("Deleting recent messages".to_string())
        .await?;

    for messages in initial_bulk.chunks(100) {
        ctx.channel_id().delete_messages(ctx, messages).await?;
    }

    reporter.report("Deleting old messages".to_string()).await?;

    let mut reporter = CountingReporter::new_with_defaults(
        Duration::from_secs(5),
        20,
        reporter.into_report_function(),
    );

    reporter.current_count = initial_bulk.len();
    reporter.last_count = initial_bulk.len();

    if let Some(message) = next_message {
        message.delete(ctx).await?;
        reporter.current_count += 1;

        // let mut bulk = vec![];
        while let Some(message) = messages.next().await {
            let message = message?;
            message.delete(ctx).await?;

            reporter.current_count += 1;
            reporter
                .report(format!(
                    "Deleting messages: {}/unknown",
                    reporter.current_count
                ))
                .await?;
            // bulk.push(channel_id.delete_message(ctx, message_id));
            // if bulk.len() > BATCH_SIZE {
            //     futures::future::join_all(bulk).await;
            //     bulk = vec![];
            //     reporter.current_count += BATCH_SIZE;
            //     reporter
            //         .report(format!(
            //             "Deleting messages: {}/unknown",
            //             reporter.current_count
            //         ))
            //         .await?;
            // }
        }
        // reporter.current_count += bulk.len();
        // reporter
        //     .report(format!(
        //         "Wiping messages: {}/unknown",
        //         reporter.current_count
        //     ))
        //     .await?;
        // futures::future::join_all(bulk).await;
    }

    reporter.force_report("Wiping finished".to_string()).await?;

    Ok(())
}
