use runir::*;
use semver::Version;
use serde::Serialize;
use std::str::FromStr;
use std::{borrow::Cow, collections::BTreeSet, fmt::Display, path::PathBuf};

pub use utils::LATEST_VERSION;
pub use utils::parse_name;

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

impl FromStr for Name {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        utils::parse_name(s)
    }
}

pub mod utils {
    use runir::util::*;
    use crate::Error;
    use semver::{BuildMetadata, Prerelease, Version};
    use std::{collections::BTreeSet, path::PathBuf, str::FromStr};

    use super::Name;
    use crate::Result;

    /// Type-alias for a plugin ref
    type PluginRefStr = Delimitted<'/', String, 2>;
    /// Type-alias for a full plugin ref (without the package prefix)
    type FullPluginRefStr = Delimitted<'@', String, 2>;
    /// Type-alias for a plugin.module pair
    type PluginModuleStr = Delimitted<'.', String, 2>;

    /// Version that represents the "latest" version
    pub const LATEST_VERSION: Version = Version {
        major: 0,
        minor: 0,
        patch: 0,
        pre: Prerelease::EMPTY,
        build: BuildMetadata::EMPTY,
    };

    /// Parses a `Name` from a string
    ///
    /// If the name is not a full reference, `LATEST_VERSION` is used as the version.
    ///
    /// The framework will always default to the current framework that is parsing the string.
    pub fn parse_name(name: &str) -> Result<Name> {
        let mut iter = PluginRefStr::from_str(name).expect("should be infallible");
        match iter.next().zip(iter.next()) {
            Some((package, plugin_ref)) if name.contains("@") => {
                let mut plugin_ref = FullPluginRefStr::from_str(&plugin_ref).unwrap();
                match plugin_ref
                    .next()
                    .zip(plugin_ref.next().and_then(|v| Version::from_str(&v).ok()))
                {
                    Some((module_plugin, version)) => {
                        let mut plugin_module = PluginModuleStr::from_str(&module_plugin).unwrap();
                        match plugin_module.next().zip(plugin_module.next()) {
                            Some((module, plugin)) => {
                                let path = PathBuf::from(&package)
                                    .join(version.to_string())
                                    .join(&module)
                                    .join(&plugin);
                                Ok(Name {
                                    package,
                                    version,
                                    module,
                                    plugin,
                                    path,
                                    qualifiers: vec![],
                                    framework: (env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
                                    matchers: BTreeSet::new(),
                                }
                                .init_matchers())
                            }
                            None => Err(Error::IncompletePluginName),
                        }
                    }
                    None => Err(Error::IncompletePluginName),
                }
            }
            Some((package, plugin_ref)) => {
                let mut plugin_module = PluginModuleStr::from_str(&plugin_ref).unwrap();
                match plugin_module.next().zip(plugin_module.next()) {
                    Some((module, plugin)) => {
                        let path = PathBuf::from(&package)
                            .join(LATEST_VERSION.to_string())
                            .join(&module)
                            .join(&plugin);
                        Ok(Name {
                            package,
                            version: LATEST_VERSION,
                            module,
                            plugin,
                            path,
                            qualifiers: vec![],
                            framework: (env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
                            matchers: BTreeSet::new(),
                        }
                        .init_matchers())
                    }
                    None => Err(Error::IncompletePluginName),
                }
            }
            None => Err(Error::IncompletePluginName),
        }
    }

    #[test]
    fn test_parse_name() {
        use crate::Plugin;

        let name = crate::tests::TestPlugin::name();
        let full_name = parse_name(&name.full_plugin_ref());
        assert_eq!("reality/0.1.0/tests/testplugin", full_name.unwrap().path().to_string_lossy());

        let name = parse_name(&name.plugin_ref());
        assert_eq!("reality/0.0.0/tests/testplugin", name.unwrap().path().to_string_lossy());
    }

    #[test]
    fn test_plugin_ref_delimitted_impl() {
        use crate::Plugin;

        let name = crate::tests::TestPlugin::name();
        let mut iter = PluginRefStr::from_str(&name.plugin_ref()).unwrap();
        let package = iter.next();
        let module_plugin = iter.next();
        assert_eq!(Some("reality".to_string()), package);
        assert_eq!(Some("tests.testplugin".to_string()), module_plugin);

        let mut iter = PluginRefStr::from_str(&name.full_plugin_ref()).unwrap();
        let package = iter.next().unwrap();
        let plugin_ref = iter.next().unwrap();
        let mut plugin_ref = FullPluginRefStr::from_str(&plugin_ref).unwrap();
        let plugin_module = plugin_ref.next();
        let plugin_version = plugin_ref.next();
        assert_eq!("reality", package);
        assert_eq!(Some("tests.testplugin".to_string()), plugin_module);
        assert_eq!(Some("0.1.0".to_string()), plugin_version);

        let plugin_module = plugin_module.unwrap();
        let mut plugin_module = PluginModuleStr::from_str(&plugin_module).unwrap();
        let module = plugin_module.next();
        let plugin = plugin_module.next();
        assert_eq!(Some("tests".to_string()), module);
        assert_eq!(Some("testplugin".to_string()), plugin);
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
