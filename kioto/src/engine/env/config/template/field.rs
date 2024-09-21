use std::str::FromStr;
use serde::{de::Visitor, Deserialize, Serialize};

/// Container type for a field that is either a template string or the actual value,
/// When deserializing, if the value is a string that contains mustache tags and can be compiled into mustache template,
/// then this will deserialize into a mustache template
/// 
/// Otherwise, if the inner type does not contain mustache tags, then it will try to deserialize as the inner type T via `FromStr`.
/// 
/// When serializing, if the inner value is set, than the inner value will be serialized, otherwise the template will be serialized instead.
pub struct TemplateField<T> {
    inner: Option<T>,
    template: Option<String>
}

impl<T> TemplateField<T> {
    /// Returns the inner value or None if it hasn't been set
    #[inline]
    pub fn as_inner(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    /// Returns the inner value wrapped as a Result<T, ()> in order to map an error
    #[inline]
    pub fn try_as_inner(&self) -> Result<&T, ()> {
        match self.inner.as_ref() {
            Some(inner) => Ok(inner),
            None => Err(()),
        }
    }
}

impl<T> From<T> for TemplateField<T> {
    fn from(value: T) -> Self {
        TemplateField { inner: Some(value), template: None }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for TemplateField<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(inner) = self.inner.as_ref() {
            write!(f, "{inner}")
        } else if let Some(template) = self.template.as_ref() {
            write!(f, "{template}")
        } else {
            Ok(())
        }
    }
}

impl<'de, T: FromStr> Visitor<'de> for TemplateField<T>
where 
    T::Err: std::fmt::Display
{
    type Value = TemplateField<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Expecting either a template string in mustache format or string version of type {}", std::any::type_name::<T>())
    }

    fn visit_string<E>(mut self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
    {
        if v.contains("{{") && v.contains("}}") {
            let _ = mustache::compile_str(&v).map_err(|e| serde::de::Error::custom(e.to_string()))?;
            self.template = Some(v);
            Ok(self)
        } else {
            let value: T = T::from_str(&v).map_err(|e| serde::de::Error::custom(e.to_string()))?;
            self.inner = Some(value);
            Ok(self)
        }
    }
}

impl<'de, T: FromStr> Deserialize<'de> for TemplateField<T> 
where 
    T::Err: std::fmt::Display
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        deserializer.deserialize_string(TemplateField::default())
    }
}

impl<T: Serialize> Serialize for TemplateField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        if let Some(inner) = self.inner.as_ref() {
            inner.serialize(serializer)
        } else if let Some(template) = self.template.as_ref() {
            serializer.serialize_str(template.as_str())
        } else {
            Err(serde::ser::Error::custom("Could not serialize empty template field"))
        }
    }
}

impl<T> Default for TemplateField<T> {
    fn default() -> Self {
        Self { inner: None, template: None }
    }
}