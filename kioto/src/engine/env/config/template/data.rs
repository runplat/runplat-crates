use crate::engine::Metadata;
use reality::runir;
use reality::BincodeContent;
use reality::Content;
use reality::Repr;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;

#[derive(Repr)]
pub struct TemplateData {
    data: InputData,
}

impl TemplateData {
    /// Apply template data to a plugin
    pub fn apply<P: Serialize + DeserializeOwned + Metadata>(
        &self,
        plugin: &P,
    ) -> std::io::Result<P> {
        match &self.data {
            InputData::Json(map) => plugin.apply_template_json_data(map),
            InputData::Toml(table) => plugin.apply_template_toml_data(table),
        }
    }
}

impl From<serde_json::Value> for TemplateData {
    fn from(value: serde_json::Value) -> Self {
        value
            .as_object()
            .map(|o| TemplateData::from(o.clone()))
            .unwrap_or(TemplateData::from(Map::new()))
    }
}

impl From<serde_json::Map<String, serde_json::Value>> for TemplateData {
    fn from(value: serde_json::Map<String, serde_json::Value>) -> Self {
        Self {
            data: InputData::Json(value),
        }
    }
}

impl From<toml::Table> for TemplateData {
    fn from(value: toml::Table) -> Self {
        Self {
            data: InputData::Toml(value),
        }
    }
}

#[derive(Serialize, Deserialize)]
enum InputData {
    Json(serde_json::Map<String, serde_json::Value>),
    Toml(toml::Table),
}

impl Content for TemplateData {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        BincodeContent::new(&self.data).unwrap().state_uuid()
    }
}
