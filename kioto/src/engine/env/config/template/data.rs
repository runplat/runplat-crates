use crate::engine::Metadata;
use serde::de::DeserializeOwned;
use serde::Serialize;
use reality::plugin::RequestData;

/// Wrapper over request data the can be used to apply a template to a plugin
pub struct TemplateData {
    data: RequestData,
}

impl TemplateData {
    /// Apply template data to a plugin
    pub fn apply<P: Serialize + DeserializeOwned + Metadata>(
        &self,
        plugin: &P,
    ) -> std::io::Result<P> {
        match &self.data {
            RequestData::Json(map) => plugin.apply_template_json_data(map),
            RequestData::Toml(table) => plugin.apply_template_toml_data(table),
            _ => {
                Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Unsupported request data type"))
            }
        }
    }
}

impl From<RequestData> for TemplateData {
    fn from(value: RequestData) -> Self {
        Self { data: value }
    }
}

impl From<serde_json::Value> for TemplateData {
    fn from(value: serde_json::Value) -> Self {
        Self {
            data: RequestData::from(value)
        }
    }
}

impl From<serde_json::Map<String, serde_json::Value>> for TemplateData {
    fn from(value: serde_json::Map<String, serde_json::Value>) -> Self {
        Self {
            data: RequestData::from(value),
        }
    }
}

impl From<toml::Table> for TemplateData {
    fn from(value: toml::Table) -> Self {
        Self {
            data: RequestData::from(value),
        }
    }
}
