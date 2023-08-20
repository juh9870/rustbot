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
        channel: ChannelId,
    ) -> anyhow::Result<MessagesRange> {
        if self.before.is_some() {
            return Ok(*self);
        }

        let last_msg = channel
            .messages(http, |b| b.limit(1))
            .await?
            .first()
            .map(|e| e.id);
        let mut cloned = *self;

        cloned.before = last_msg;

        Ok(cloned)
    }

    pub fn unbounded() -> Self {
        Self {
            before: None,
            after: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SmartMessagesIter<H: AsRef<Http>> {
    http: H,
    channel_id: ChannelId,
    buffer: Vec<Message>,
    before: Option<MessageId>,
    tried_fetch: bool,
    range: MessagesRange,
}

impl<H: AsRef<Http>> SmartMessagesIter<H> {
    fn new(http: H, channel_id: ChannelId, range: MessagesRange) -> SmartMessagesIter<H> {
        SmartMessagesIter {
            http,
            channel_id,
            buffer: Vec::new(),
            before: range.before,
            tried_fetch: false,
            range,
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
    /// element of the buffer and the newest message is the last.
    ///
    /// [`Message`]: crate::model::channel::Message
    async fn refresh(&mut self) -> Result<()> {
        // Number of messages to fetch.
        let grab_size = 100;

        // If `self.before` is not set yet, we can use `.messages` to fetch
        // the last message after very first fetch from last.
        self.buffer = self
            .channel_id
            .messages(&self.http, |b| {
                if let Some(before) = self.before {
                    b.before(before);
                }

                match self.range.after {
                    None => b.limit(grab_size),
                    Some(after) => b.limit(grab_size).after(after),
                }
            })
            .await?;

        self.buffer.reverse();

        self.before = self.buffer.first().map(|m| m.id);

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
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use serenity::model::id::ChannelId;
    /// # use serenity::http::Http;
    /// #
    /// # async fn run() {
    /// # let channel_id = ChannelId::default();
    /// # let ctx = Http::new("token");
    /// use serenity::futures::StreamExt;
    /// use serenity::model::channel::MessagesIter;
    ///
    /// let mut messages = SmartMessagesIter::<Http>::stream(&ctx, channel_id).boxed();
    /// while let Some(message_result) = messages.next().await {
    ///     match message_result {
    ///         Ok(message) => println!("{} said \"{}\"", message.author.name, message.content,),
    ///         Err(error) => eprintln!("Uh oh! Error: {}", error),
    ///     }
    /// }
    /// # }
    /// ```
    pub fn stream(
        http: impl AsRef<Http>,
        channel_id: ChannelId,
        range: MessagesRange,
    ) -> impl Stream<Item = Result<Message>> {
        let init_state = SmartMessagesIter::new(http, channel_id, range);

        futures::stream::unfold(init_state, |mut state| async {
            if state.buffer.is_empty() && state.before.is_some() || !state.tried_fetch {
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
    channel_id: ChannelId,
    range: MessagesRange,
) -> impl Stream<Item = Result<Message>> {
    SmartMessagesIter::<H>::stream(http, channel_id, range)
}
