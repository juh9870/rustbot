use futures::Stream;
use poise::serenity_prelude::*;

#[derive(Clone, Debug)]
pub struct SmartMessagesIter<H: AsRef<Http>> {
    http: H,
    channel_id: ChannelId,
    buffer: Vec<Message>,
    before: Option<MessageId>,
    tried_fetch: bool,
}

impl<H: AsRef<Http>> SmartMessagesIter<H> {
    fn new(http: H, channel_id: ChannelId, before: Option<MessageId>) -> SmartMessagesIter<H> {
        SmartMessagesIter {
            http,
            channel_id,
            buffer: Vec::new(),
            before,
            tried_fetch: false,
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

                b.limit(grab_size)
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
    /// let mut messages = MessagesIter::<Http>::stream(&ctx, channel_id).boxed();
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
        before: Option<MessageId>,
    ) -> impl Stream<Item = Result<Message>> {
        let init_state = SmartMessagesIter::new(http, channel_id, before);

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
