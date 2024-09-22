use std::{collections::BTreeMap, sync::{Arc, RwLock}};

/// Struct containing request state for plugin calls
/// 
/// This is stored centrally w/ a State object so to consolidate,
/// statefulness away from the Call objects themselves. Instead the call
/// objects can check state for requests and remove requests from a single location
#[derive(Clone, Default)]
pub struct Requests {
    data: Arc<RwLock<BTreeMap<u64, RequestData>>>
}

/// Enum of supported request data that can be accepted by plugins
pub enum RequestData {
    Toml(toml::Table),
    Json(serde_json::Map<String, serde_json::Value>),
    Item(runir::store::Item),
    Empty,
}

impl Requests {
    /// Sends a request to a dest handle
    /// 
    /// Returns an error if previous data has already been set for the handle, or if in between
    /// acquiring the write lock, an entry was written before this function could write.
    pub fn send(&self, dest: u64, data: impl Into<RequestData>) -> crate::Result<()> {
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
    pub fn receive(&self, commit: u64) -> RequestData {
        let mut g = match self.data.write() {
            Ok(g) => g,
            Err(err) => err.into_inner(),
        };
        g.remove(&commit).unwrap_or(RequestData::Empty)
    }
}

impl From<toml::Table> for RequestData {
    fn from(value: toml::Table) -> Self {
        Self::Toml(value)
    }
}

impl From<serde_json::Value> for RequestData {
    fn from(value: serde_json::Value) -> Self {
        value.as_object().map(|m| {
            RequestData::Json(m.clone())
        }).unwrap_or(RequestData::Empty)
    }
}

impl From<serde_json::Map<String, serde_json::Value>> for RequestData {
    fn from(value: serde_json::Map<String, serde_json::Value>) -> Self {
        Self::Json(value)
    }
}

impl From<runir::store::Item> for RequestData {
    fn from(value: runir::store::Item) -> Self {
        Self::Item(value)
    }
}

impl From<()> for RequestData {
    fn from(_: ()) -> Self {
        Self::Empty
    }
}
