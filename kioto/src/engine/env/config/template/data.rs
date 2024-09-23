use crate::engine::Metadata;
use serde::de::DeserializeOwned;
use serde::Serialize;
use reality::plugin::MessageData;

/// Wrapper over request data the can be used to apply a template to a plugin
pub struct TemplateData {
    data: MessageData,
}

impl TemplateData {
    /// Apply template data to a plugin
    pub fn apply<P: Serialize + DeserializeOwned + Metadata>(
        &self,
        plugin: &P,
    ) -> std::io::Result<P> {
        match &self.data {
            MessageData::Json(map) => plugin.apply_template_json_data(map),
            MessageData::Toml(table) => plugin.apply_template_toml_data(table),
            _ => {
                Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Unsupported request data type"))
            }
        }
    }
}

impl From<MessageData> for TemplateData {
    fn from(value: MessageData) -> Self {
        Self { data: value }
    }
}

impl From<serde_json::Value> for TemplateData {
    fn from(value: serde_json::Value) -> Self {
        Self {
            data: MessageData::from(value)
        }
    }
}

impl From<serde_json::Map<String, serde_json::Value>> for TemplateData {
    fn from(value: serde_json::Map<String, serde_json::Value>) -> Self {
        Self {
            data: MessageData::from(value),
        }
    }
}

impl From<toml::Table> for TemplateData {
    fn from(value: toml::Table) -> Self {
        Self {
            data: MessageData::from(value),
        }
    }
}
