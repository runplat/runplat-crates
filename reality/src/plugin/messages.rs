use bytes::Bytes;
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use tracing::debug;

/// Type-alias for a json map
pub type JsonMap = serde_json::Map<String, serde_json::Value>;

/// Struct containing request state for plugin calls
///
/// This is stored centrally w/ a State object so to consolidate,
/// statefulness away from the Call objects themselves. Instead the call
/// objects can check state for requests and remove requests from a single location
#[derive(Clone, Default)]
pub struct Broker {
    data: Arc<RwLock<BTreeMap<u64, MessageData>>>,
}
/// Enum of supported request data that can be accepted by plugins
#[derive(Default)]
pub enum MessageData {
    /// Message data is a TOML table
    Toml(toml::Table),
    /// Message data is a JSON map
    Json(serde_json::Map<String, serde_json::Value>),
    /// Message data is binary data
    Bytes(Bytes),
    /// Message data is a store item
    Item(runir::store::Item),
    /// Empty message data
    #[default]
    Empty,
}

impl MessageData {
    /// Returns true if the message data is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self, MessageData::Empty)
    }

    /// Returns true if the message data is json
    #[inline]
    pub fn is_json(&self) -> bool {
        matches!(self, MessageData::Json(..))
    }

    /// Returns true if the message data is toml
    #[inline]
    pub fn is_toml(&self) -> bool {
        matches!(self, MessageData::Toml(..))
    }

    /// Returns true if the message is a runir::store::Item
    #[inline]
    pub fn is_item(&self) -> bool {
        matches!(self, MessageData::Item(..))
    }

    /// Returns true if the message is a Bytes object
    #[inline]
    pub fn is_bytes(&self) -> bool {
        matches!(self, MessageData::Bytes(..))
    }

    /// If message data is Bytes, returns a reference to the Bytes
    #[inline]
    pub fn as_bytes(&self) -> Option<&Bytes> {
        if let MessageData::Bytes(bytes) = self {
            Some(bytes)
        } else {
            None
        }
    }

    /// If message data is an runir::store::Item, returns a reference to the Item
    #[inline]
    pub fn as_item(&self) -> Option<&runir::store::Item> {
        if let MessageData::Item(item) = self {
            Some(item)
        } else {
            None
        }
    }

    /// If message data is JSON, returns a reference to the inner JSON value
    #[inline]
    pub fn as_json(&self) -> Option<&JsonMap> {
        if let MessageData::Json(map) = self {
            Some(map)
        } else {
            None
        }
    }

    /// If message data is TOML, returns a reference to the inner TOML value
    #[inline]
    pub fn as_toml(&self) -> Option<&toml::Table> {
        if let MessageData::Toml(table) = self {
            Some(table)
        } else {
            None
        }
    }
}

impl Broker {
    /// Sends a request to a dest handle
    ///
    /// Returns an error if previous data has already been set for the handle, or if in between
    /// acquiring the write lock, an entry was written before this function could write.
    pub fn send(&self, dest: u64, data: impl Into<MessageData>) -> crate::Result<()> {
        debug!("Send data to {dest:x}");
        let g = match self.data.read() {
            Ok(g) => g,
            Err(err) => err.into_inner(),
        };

        if g.contains_key(&dest) {
            Err(crate::Error::PreviousUnhandledRequest)
        } else {
            drop(g);
            let mut g = match self.data.write() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };

            if let Some(previous) = g.insert(dest, data.into()) {
                let _ = g.remove(&dest);
                g.insert(dest, previous);

                Err(crate::Error::WriteRequestRaceCondition)
            } else {
                Ok(())
            }
        }
    }

    /// Receive a request for a handle
    #[inline]
    pub fn receive(&self, commit: u64) -> MessageData {
        debug!("Receive data for {commit:x}");
        let mut g = match self.data.write() {
            Ok(g) => g,
            Err(err) => err.into_inner(),
        };
        g.remove(&commit).unwrap_or(MessageData::Empty)
    }
}

impl From<toml::Table> for MessageData {
    fn from(value: toml::Table) -> Self {
        Self::Toml(value)
    }
}

impl From<serde_json::Value> for MessageData {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Array(vec) => {
                let mut map = serde_json::Map::new();
                map.insert("[]".to_string(), vec.into());
                MessageData::Json(map)
            }
            serde_json::Value::Object(map) => MessageData::Json(map),
            _ => MessageData::Empty,
        }
    }
}

impl From<serde_json::Map<String, serde_json::Value>> for MessageData {
    fn from(value: serde_json::Map<String, serde_json::Value>) -> Self {
        Self::Json(value)
    }
}

impl From<runir::store::Item> for MessageData {
    fn from(value: runir::store::Item) -> Self {
        Self::Item(value)
    }
}

impl From<Bytes> for MessageData {
    fn from(value: Bytes) -> Self {
        MessageData::Bytes(value)
    }
}

impl From<()> for MessageData {
    fn from(_: ()) -> Self {
        Self::Empty
    }
}
