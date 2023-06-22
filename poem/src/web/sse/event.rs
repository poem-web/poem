use std::fmt::{self, Display, Formatter};

/// An "event", either an incoming message or some meta-action that needs to be
/// applied to the stream.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(docsrs, doc(cfg(feature = "sse")))]
pub enum Event {
    /// An incoming message.
    Message {
        /// The ID of this event.
        ///
        /// See also the [Server-Sent Events spec](https://html.spec.whatwg.org/multipage/server-sent-events.html#concept-event-stream-last-event-id).
        id: String,
        /// The event type. Defaults to "message" if no event name is provided.
        event: String,
        /// The data for this event.
        data: String,
    },
    /// Set the _reconnection time_.
    ///
    /// See also the [Server-Sent Events spec](https://html.spec.whatwg.org/multipage/server-sent-events.html#concept-event-stream-reconnection-time).
    Retry {
        /// The new reconnection time in milliseconds.
        retry: u64,
    },
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Event::Message { id, event, data } => {
                if !id.is_empty() {
                    writeln!(f, "id: {}", &id)?;
                }
                if !event.is_empty() && event != "message" {
                    writeln!(f, "event: {}", &event)?;
                }
                for line in data.lines() {
                    writeln!(f, "data: {line}")?;
                }
                writeln!(f)?;
                Ok(())
            }
            Event::Retry { retry } => {
                writeln!(f, "retry: {retry}")?;
                writeln!(f)
            }
        }
    }
}

impl Event {
    /// Create a server-sent event message.
    #[must_use]
    pub fn message(data: impl Into<String>) -> Self {
        Event::Message {
            id: String::new(),
            event: String::new(),
            data: data.into(),
        }
    }

    /// Set the id of the message. If the event is not a message type, there
    /// will be no effect.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        if let Event::Message { id: msg_id, .. } = &mut self {
            *msg_id = id.into();
        }
        self
    }

    /// Set the event type of the message. If the event is not a message type,
    /// there will be no effect.
    #[must_use]
    pub fn event_type(mut self, event: impl Into<String>) -> Self {
        if let Event::Message {
            event: msg_event, ..
        } = &mut self
        {
            *msg_event = event.into();
        }
        self
    }

    /// Create a message that configures the retry timeout.
    #[must_use]
    pub fn retry(time: u64) -> Self {
        Event::Retry { retry: time }
    }
}
