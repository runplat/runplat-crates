use std::{collections::BTreeMap, str::FromStr};

use serde::{de::DeserializeOwned, Serialize};
use toml_edit::value;
use tracing::debug;

/// Contains a map of fields and their respective expected hash settings
/// for the tags each template is using
/// 
/// Uses `mustache {{}}` as the template implementation
pub struct TemplateMap<'a> {
    /// Source of template map
    fields: &'a BTreeMap<String, toml::Table>
}

impl<'a> TemplateMap<'a> {
    /// Apply toml data to map of template fields from input type
    pub fn apply_toml<S: Serialize + DeserializeOwned>(&self, input: &S, data: &toml::Table) -> std::io::Result<S> {
        let ser = toml::to_string(input).map_err(convert_toml_err_to_io_err)?;

        let mut doc = toml_edit::DocumentMut::from_str(&ser).map_err(convert_toml_edit_err_to_io_err)?;

        for (k, _v) in self.fields.iter() {
            if let Some(field) = doc.get(&k).and_then(|v| v.as_str()) {
                let template = mustache::compile_str(field).map_err(convert_mustach_err_to_io_err)?;

                if let Some(input) = data[k].as_table() {
                    // TODO: Use _v to validate inputs

                    let render = template.render_to_string(input).map_err(convert_mustach_err_to_io_err)?;
                    if let Some(old) = doc.insert(&k, value(&render)) {
                        debug!("Applied template {old} -> {render}");
                    }

                } else {
                    return Err(missing_data_for_field(k));
                }
            }
        }

        toml::from_str(doc.to_string().as_str()).map_err(convert_toml_de_err_to_io_err)
    }

    /// Apply json data to map of template fields from input type
    pub fn apply_json<S: Serialize + DeserializeOwned>(&self, input: &S, data: &serde_json::Map<String, serde_json::Value>) -> std::io::Result<S> {
        let ser = toml::to_string(input).map_err(convert_toml_err_to_io_err)?;

        let mut doc = toml_edit::DocumentMut::from_str(&ser).map_err(convert_toml_edit_err_to_io_err)?;

        for (k, _v) in self.fields.iter() {
            if let Some(field) = doc.get(&k).and_then(|v| v.as_str()) {
                let template = mustache::compile_str(field).map_err(convert_mustach_err_to_io_err)?;

                if let Some(input) = data[k].as_object() {
                    // TODO: Use _v to validate inputs

                    let render = template.render_to_string(input).map_err(convert_mustach_err_to_io_err)?;
                    if let Some(old) = doc.insert(&k, value(&render)) {
                        debug!("Applied template {old} -> {render}");
                    }
                } else {
                    return Err(missing_data_for_field(k));
                }
            }
        }

        toml::from_str(doc.to_string().as_str()).map_err(convert_toml_de_err_to_io_err)
    }
}

impl<'a> From<&'a BTreeMap<String, toml::Table>> for TemplateMap<'a> {
    fn from(value: &'a BTreeMap<String, toml::Table>) -> Self {
        Self { fields: value }
    }
}

fn missing_data_for_field(k: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, format!("Missing input data for field `{k}`"))
}

fn convert_toml_err_to_io_err(err: toml::ser::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())
}

fn convert_toml_de_err_to_io_err(err: toml::de::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())
}

fn convert_toml_edit_err_to_io_err(err: toml_edit::TomlError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())
}

fn convert_mustach_err_to_io_err(err: mustache::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;
    use toml::toml;
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestSubject {
        url: String
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_apply_template_map_with_toml_data() {
        let template_map: toml::Table = toml! {
            host = ""
            path = ""
        };

        let mut fields = BTreeMap::new();
        fields.insert("url".to_string(), template_map);

        let subject = TestSubject {
            url: r"https://{{host}}/{{path}}".to_string()
        };

        let template_map = TemplateMap::from(&fields);
        let input: toml::Table = toml! {
            [url]
            host = "example.com"
            path = "test_path"
        };
        let result = template_map.apply_toml(&subject, &input).unwrap();
        assert_eq!("https://example.com/test_path", result.url);
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_apply_template_map_with_json_data() {
        let template_map: toml::Table = toml! {
            host = ""
            path = ""
        };

        let mut fields = BTreeMap::new();
        fields.insert("url".to_string(), template_map);

        let subject = TestSubject {
            url: r"https://{{host}}/{{path}}".to_string()
        };

        let template_map = TemplateMap::from(&fields);
        let input = json! ({
            "url": {
                "host": "example.com",
                "path": "test_path"
            }
        }).as_object().cloned();

        let result = template_map.apply_json(&subject, &input.unwrap()).unwrap();
        assert_eq!("https://example.com/test_path", result.url);
    }
}