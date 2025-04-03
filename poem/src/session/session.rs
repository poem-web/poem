use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

use parking_lot::RwLock;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{FromRequest, Request, RequestBody, Result};

/// Status of the Session.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    /// Indicates that the session state has changed.
    Changed,

    /// Indicates that the session state needs to be cleared.
    Purged,

    /// Indicates that the session TTL(time-to-live) needs to be reset.
    Renewed,

    /// Indicates that the session state is unchanged.
    Unchanged,
}

struct SessionInner {
    status: SessionStatus,
    entries: BTreeMap<String, Value>,
}

/// Session
#[derive(Clone)]
pub struct Session {
    inner: Arc<RwLock<SessionInner>>,
}

impl Debug for Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let inner = self.inner.read();
        f.debug_struct("Session")
            .field("status", &inner.status)
            .field("entries", &inner.entries)
            .finish()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Session {
    /// Creates a new session instance.
    ///
    /// The default status is [`SessionStatus::Unchanged`].
    pub(crate) fn new(entries: BTreeMap<String, Value>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(SessionInner {
                status: SessionStatus::Unchanged,
                entries,
            })),
        }
    }

    /// Get a value from the session.
    pub fn get<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        let inner = self.inner.read();
        inner
            .entries
            .get(name)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// Sets a key-value pair into the session.
    pub fn set(&self, name: &str, value: impl Serialize) {
        let mut inner = self.inner.write();
        if inner.status != SessionStatus::Purged {
            if let Ok(value) = serde_json::to_value(&value) {
                inner.entries.insert(name.to_string(), value);
                if inner.status != SessionStatus::Renewed {
                    inner.status = SessionStatus::Changed;
                }
            }
        }
    }

    /// Remove value from the session.
    pub fn remove(&self, name: &str) {
        let mut inner = self.inner.write();
        if inner.status != SessionStatus::Purged {
            inner.entries.remove(name);
            if inner.status != SessionStatus::Renewed {
                inner.status = SessionStatus::Changed;
            }
        }
    }

    /// Returns `true` is this session does not contain any values, otherwise it
    /// returns `false`.
    pub fn is_empty(&self) -> bool {
        let inner = self.inner.read();
        inner.entries.is_empty()
    }

    /// Get all raw key-value data from the session
    pub fn entries(&self) -> BTreeMap<String, Value> {
        let inner = self.inner.read();
        inner.entries.clone()
    }

    /// Clear the session.
    pub fn clear(&self) {
        let mut inner = self.inner.write();
        if inner.status != SessionStatus::Purged {
            inner.entries.clear();
            if inner.status != SessionStatus::Renewed {
                inner.status = SessionStatus::Changed;
            }
        }
    }

    /// Renews the session key, assigning existing session state to new key.
    pub fn renew(&self) {
        let mut inner = self.inner.write();
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Renewed;
        }
    }

    /// Removes session both client and server side.
    pub fn purge(&self) {
        let mut inner = self.inner.write();
        if inner.status != SessionStatus::Purged {
            inner.entries.clear();
            inner.status = SessionStatus::Purged;
        }
    }

    /// Returns the status of this session.
    pub fn status(&self) -> SessionStatus {
        let inner = self.inner.read();
        inner.status
    }
}

impl<'a> FromRequest<'a> for &'a Session {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req
            .extensions()
            .get::<Session>()
            .expect("To use the `Session` extractor, the `CookieSession` middleware is required."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_session() {
        let session = Session::default();
        session.set("a", 1);

        assert_eq!(session.status(), SessionStatus::Changed);
        assert_eq!(
            session.entries().into_iter().collect::<Vec<_>>(),
            vec![("a".to_string(), 1.into())]
        );

        session.set("b", 2);
        assert_eq!(session.status(), SessionStatus::Changed);
        assert_eq!(
            session.entries().into_iter().collect::<Vec<_>>(),
            vec![("a".to_string(), 1.into()), ("b".to_string(), 2.into())]
        );

        session.renew();
        session.set("c", 3);
        assert_eq!(session.status(), SessionStatus::Renewed);
        assert_eq!(
            session.entries().into_iter().collect::<Vec<_>>(),
            vec![
                ("a".to_string(), 1.into()),
                ("b".to_string(), 2.into()),
                ("c".to_string(), 3.into())
            ]
        );

        session.remove("c");
        assert_eq!(session.status(), SessionStatus::Renewed);
        assert_eq!(
            session.entries().into_iter().collect::<Vec<_>>(),
            vec![("a".to_string(), 1.into()), ("b".to_string(), 2.into()),]
        );

        session.clear();
        assert_eq!(session.status(), SessionStatus::Renewed);
        assert_eq!(session.entries().into_iter().collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn purge_session() {
        let session = Session::default();

        session.set("a", 1);
        session.set("b", 2);
        assert_eq!(session.status(), SessionStatus::Changed);
        assert_eq!(
            session.entries().into_iter().collect::<Vec<_>>(),
            vec![("a".to_string(), 1.into()), ("b".to_string(), 2.into())]
        );

        session.purge();
        session.set("c", 3);
        assert_eq!(session.status(), SessionStatus::Purged);
        assert_eq!(session.entries().into_iter().collect::<Vec<_>>(), vec![]);

        session.clear();
        assert_eq!(session.status(), SessionStatus::Purged);
        assert_eq!(session.entries().into_iter().collect::<Vec<_>>(), vec![]);

        session.set("d", 4);
        assert_eq!(session.status(), SessionStatus::Purged);
        assert_eq!(session.entries().into_iter().collect::<Vec<_>>(), vec![]);

        session.remove("d");
        assert_eq!(session.status(), SessionStatus::Purged);
        assert_eq!(session.entries().into_iter().collect::<Vec<_>>(), vec![]);
    }
}
