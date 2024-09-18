use runir::*;
use semver::Version;
use serde::Serialize;
use std::{borrow::Cow, collections::BTreeSet, fmt::Display, path::PathBuf};

use crate::BincodeContent;

use super::Plugin;

/// Type-alias for a Plugin reference string
pub type PluginRef<'a> = Cow<'a, str>;

/// Struct containing name data
#[derive(Debug, Serialize, Clone, PartialEq, PartialOrd)]
pub struct Name {
    pub(crate) package: String,
    pub(crate) version: Version,
    pub(crate) module: String,
    pub(crate) plugin: String,
    pub(crate) path: PathBuf,
    pub(crate) qualifiers: Vec<String>,
    pub(crate) framework: (&'static str, &'static str),
    pub(crate) matchers: BTreeSet<String>,
}

impl Name {
    /// Creates a new name and generates a path to reference the type with
    ///
    /// ## Path format
    ///
    /// The format of the path is `{package-name}/{package-version}/{upper-most-module}/{type-name}`
    #[inline]
    pub fn new<T>() -> Name
    where
        T: Plugin + ?Sized,
    {
        let version = T::version();
        let mut fq_ty_name = std::any::type_name::<T>()
            .split("::")
            .map(|p| p.to_lowercase());

        let package = fq_ty_name.next();
        let module = fq_ty_name.next();
        let mut rest = fq_ty_name.collect::<Vec<_>>();
        let plugin = rest.pop();
        let qualifiers = rest;
        match package.zip(module).zip(plugin) {
            Some(((package, module), plugin)) => {
                let path = PathBuf::from(&package)
                    .join(version.to_string())
                    .join(&module)
                    .join(&plugin);
                Name {
                    package,
                    version,
                    module,
                    plugin,
                    path,
                    qualifiers,
                    framework: T::framework(),
                    matchers: BTreeSet::new(),
                }
                .init_matchers()
            }
            _ => Name {
                package: format!("unknown"),
                version,
                module: format!("unknown"),
                plugin: uuid::Uuid::new_v4().to_string(),
                path: PathBuf::new(),
                qualifiers,
                framework: T::framework(),
                matchers: BTreeSet::new(),
            }
            .init_matchers(),
        }
    }

    /// Returns this name in a path format
    #[inline]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns this name in the full plugin reference format which includes the version
    ///
    /// **Note**: This is the "alternate" display format of `Name::to_string`
    #[inline]
    pub fn full_plugin_ref(&self) -> PluginRef {
        Cow::Owned(format!("{self:#}"))
    }

    /// Returns the short plugin reference format which does not include the version
    ///
    /// **Note**: This is the default display format of `Name::to_string`
    #[inline]
    pub fn plugin_ref(&self) -> PluginRef {
        Cow::Owned(format!("{self}"))
    }

    /// Returns name qualifiers for this plugin
    ///
    /// Name qualifiers are the symbols between the plugins type name and package name
    #[inline]
    pub fn qualifiers(&self) -> impl Iterator<Item = &str> {
        self.qualifiers.iter().map(|q| q.as_str())
    }

    /// Initializes matchers for this name
    #[inline]
    fn init_matchers(mut self) -> Self {
        self.matchers.insert(self.plugin_ref().to_string());
        self.matchers.insert(self.full_plugin_ref().to_string());
        self.matchers
            .insert(self.path().to_string_lossy().to_string());
        self
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(
                f,
                "{}/{}.{}@{}",
                self.package, self.module, self.plugin, self.version
            )
        } else {
            write!(f, "{}/{}.{}", self.package, self.module, self.plugin)
        }
    }
}

impl Repr for Name {}
impl Resource for Name {}

impl Content for Name {
    fn state_uuid(&self) -> uuid::Uuid {
        BincodeContent::new(self).unwrap().state_uuid()
    }
}

#[cfg(test)]
mod tests {
    use crate::Plugin;

    use super::Name;
    use runir::{Content, Repr, Resource};
    use semver::Version;
    use uuid::Uuid;

    struct Test;
    impl Resource for Test {}
    impl Repr for Test {}
    impl Content for Test {
        fn state_uuid(&self) -> uuid::Uuid {
            Uuid::nil()
        }
    }
    impl Plugin for Test {
        fn call(_: crate::plugin::Bind<Self>) -> crate::Result<crate::plugin::SpawnWork> {
            todo!()
        }

        fn version() -> Version {
            Version::new(0, 0, 0)
        }
    }

    #[test]
    fn test_name_formatting() {
        let name = Name::new::<Test>();
        assert_eq!("reality/plugin.test", name.to_string().as_str());
        assert_eq!("reality/plugin.test@0.0.0", format!("{name:#}"));
        assert_eq!("reality/0.0.0/plugin/test", name.path().to_string_lossy());
        assert_eq!("reality/plugin.test", name.plugin_ref().as_ref());
        assert_eq!("reality/plugin.test@0.0.0", name.full_plugin_ref().as_ref());
        assert_eq!("reality/0.0.0/plugin/test", name.path().to_string_lossy());
    }
}
