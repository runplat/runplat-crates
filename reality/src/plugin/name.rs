use runir::*;
use semver::Version;
use serde::Serialize;
use std::{fmt::Display, path::PathBuf};

/// Struct containing name data
#[derive(Debug, Serialize, Clone)]
pub struct Name {
    pub(crate) package: String,
    pub(crate) version: Version,
    pub(crate) module: String,
    pub(crate) plugin: String,
    pub(crate) path: PathBuf,
    pub(crate) qualifiers: Vec<String>
}

impl Name {
    /// Creates a new name and generates a path to reference the type with
    ///
    /// ## Path format
    ///
    /// The format of the path is `{package-name}/{package-version}/{upper-most-module}/{type-name}`
    #[inline]
    pub fn new<T>(version: semver::Version) -> Name
    where
        T: ?Sized,
    {
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
                let path = PathBuf::from(&package).join(version.to_string()).join(&module).join(&plugin);
                Name {
                    package,
                    version,
                    module,
                    plugin,
                    path,
                    qualifiers
                }
            },
            _ => {
                Name {
                    package: format!("unknown"),
                    version,
                    module: format!("unknown"),
                    plugin: uuid::Uuid::new_v4().to_string(),
                    path: PathBuf::new(),
                    qualifiers
                }
            }
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
    pub fn full_plugin_ref(&self) -> String {
        format!("{self:#}")
    }

    /// Returns the short plugin reference format which does not include the version
    ///
    /// **Note**: This is the default display format of `Name::to_string`
    #[inline]
    pub fn plugin_ref(&self) -> String {
        format!("{self}")
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

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::Name;

    #[test]
    fn test_name_formatting() {
        let name = Name::new::<String>(Version::new(0, 1, 0));
        assert_eq!("alloc/string.string", name.to_string().as_str());
        assert_eq!("alloc/string.string@0.1.0", format!("{name:#}"));
        assert_eq!("alloc/0.1.0/string/string", name.path().to_string_lossy());
        assert_eq!("alloc/string.string", name.plugin_ref());
        assert_eq!("alloc/string.string@0.1.0", name.full_plugin_ref());
        assert_eq!("alloc/0.1.0/string/string", name.path().to_string_lossy());
    }
}
