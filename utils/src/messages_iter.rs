use anyhow::Context;
use futures::Stream;
use poise::serenity_prelude::*;

#[derive(Copy, Clone, Debug)]
pub struct MessagesRange {
    pub before: Option<MessageId>,
    pub after: Option<MessageId>,
}
impl MessagesRange {
    pub async fn snapshot_for_channel<H: AsRef<Http>>(
        &self,
        http: H,
        channel_id: ChannelId,
    ) -> anyhow::Result<MessageRangeInChannel> {
        let before = if let Some(before) = self.before {
            let msg = channel_id.message(http.as_ref(), before).await.context("Failed to fetch the message specified in `before`. Does it belong to the specified channel?")?;
            Some((msg.id, msg.timestamp))
        } else {
            // find the last message in the channel to use as the `before` value
            channel_id
                .messages(http.as_ref(), GetMessages::new().limit(1))
                .await?
                .first()
                .map(|e| (e.id, e.timestamp))
        };
        let after = if let Some(after) = self.after {
            let msg = channel_id.message(http.as_ref(), after).await.context("Failed to fetch the message specified in `after`. Does it belong to the specified channel?")?;
            Some((msg.id, msg.timestamp))
        } else {
            None
        };

        Ok(MessageRangeInChannel {
            channel: channel_id,
            before,
            after,
        })
    }

    pub fn unbounded() -> Self {
        Self {
            before: None,
            after: None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MessageRangeInChannel {
    channel: ChannelId,
    before: Option<(MessageId, Timestamp)>,
    after: Option<(MessageId, Timestamp)>,
}

impl MessageRangeInChannel {
    pub fn channel_id(&self) -> ChannelId {
        self.channel
    }

    pub fn before(&self) -> Option<(MessageId, Timestamp)> {
        self.before
    }

    pub fn after(&self) -> Option<(MessageId, Timestamp)> {
        self.after
    }
}

#[derive(Clone, Debug)]
pub struct SmartMessagesIter<H: AsRef<Http>> {
    http: H,
    buffer: Vec<Message>,
    before: Option<(MessageId, Timestamp)>,
    tried_fetch: bool,
    range: MessageRangeInChannel,
    done: bool,
}

impl<H: AsRef<Http>> SmartMessagesIter<H> {
    fn new(http: H, range: MessageRangeInChannel) -> SmartMessagesIter<H> {
        SmartMessagesIter {
            http,
            buffer: Vec::new(),
            before: range.before(),
            tried_fetch: false,
            range,
            done: false,
        }
    }

    /// Fills the `self.buffer` cache with [`Message`]s.
    ///
    /// This drops any messages that were currently in the buffer. Ideally, it
    /// should only be called when `self.buffer` is empty. Additionally, this updates
    /// `self.before` so that the next call does not return duplicate items.
    ///
    /// If there are no more messages to be fetched, then this sets `self.before`
    /// as [`None`], indicating that no more calls ought to be made.
    ///
    /// If this method is called with `self.before` as None, the last 100
    /// (or lower) messages sent in the channel are added in the buffer.
    ///
    /// The messages are sorted such that the newest message is the first
    /// element of the buffer and the oldest message is the last.
    ///
    /// [`Message`]: crate::model::channel::Message
    async fn refresh(&mut self) -> Result<()> {
        // Number of messages to fetch.
        let grab_size = 100;

        let get = if let Some(before) = self.before {
            GetMessages::new().limit(grab_size).before(before.0)
        } else {
            GetMessages::new().limit(grab_size)
        };

        let http = self.http.as_ref();

        // If `self.before` is not set yet, we can use `.messages` to fetch
        // the last message after very first fetch from last.
        self.buffer = self.range.channel_id().messages(http, get).await?;

        if let Some((after, after_timestamp)) = self.range.after() {
            if let Some(truncate_at) = self
                .buffer
                .iter()
                .enumerate()
                .find(|(_, m)| m.id == after || m.timestamp < after_timestamp)
                .map(|(i, _)| i)
            {
                self.buffer.truncate(truncate_at);
                self.done = true;
            }
        }

        self.buffer.reverse();

        self.before = self.buffer.first().map(|m| (m.id, m.timestamp));

        self.tried_fetch = true;

        Ok(())
    }

    /// Streams over all the messages in a channel.
    ///
    /// This is accomplished and equivalent to repeated calls to [`ChannelId::messages`].
    /// A buffer of at most 100 messages is used to reduce the number of calls.
    /// necessary.
    ///
    /// The stream returns the newest message first, followed by older messages.
    pub fn stream(
        http: impl AsRef<Http>,
        range: MessageRangeInChannel,
    ) -> impl Stream<Item = Result<Message>> {
        let init_state = SmartMessagesIter::new(http, range);

        futures::stream::unfold(init_state, |mut state| async {
            if !state.done
                && (state.buffer.is_empty() && state.before.is_some() || !state.tried_fetch)
            {
                if let Err(error) = state.refresh().await {
                    return Some((Err(error), state));
                }
            }

            // the resultant stream goes from newest to oldest.
            state.buffer.pop().map(|entry| (Ok(entry), state))
        })
    }
}

pub fn smart_messages_iter<H: AsRef<Http>>(
    http: H,
    range: MessageRangeInChannel,
) -> impl Stream<Item = Result<Message>> {
    SmartMessagesIter::<H>::stream(http, range)
}
