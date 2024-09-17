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
}

impl Name {
    /// Creates a new name and generates a path to reference the type with
    ///
    /// ## Path format
    ///
    /// The format of the path is `{package-name}/{package-version}/{upper-most-module}/{type-name}`
    #[inline]
    pub fn new<T>(pkg_name: &str, pkg_version: &str) -> Name
    where
        T: ?Sized,
    {
        let mut name = Name {
            package: pkg_name.to_lowercase(),
            version: pkg_version
                .parse::<semver::Version>()
                .expect("should be a valid semver because cargo will not let you compile if the this value is not a valid version"),
                // NOTE: In case the above invariant is no-longer true, this is how to handle the error
                // .unwrap_or_else(|_| {
                //     let mut version = semver::Version::new(0, 0, 0);
                //     version.pre =
                //         Prerelease::new("unknown").expect("should be a valid pre-release tag");
                //     version
                // }),
            module: String::from("unknown"),
            plugin: uuid::Uuid::new_v4().to_string().to_lowercase(),
            path: PathBuf::new(),
        };

        let mut fq_ty_name = std::any::type_name::<T>()
            .split("::")
            .map(|p| p.to_lowercase())
            .skip(1); // The first component is the package name
        match fq_ty_name
            .next()
            .zip(fq_ty_name.last().map(|l| l.to_lowercase()))
        {
            Some((module, plugin)) => {
                name.module = module;
                name.plugin = plugin;
            }
            None => {}
        }

        name.path = PathBuf::from(&name.package)
            .join(name.version.to_string())
            .join(&name.module)
            .join(&name.plugin);
        name
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
    use super::Name;

    #[test]
    fn test_name_formatting() {
        let name = Name::new::<String>("reality", "string");
        assert_eq!("reality/string.string", name.to_string().as_str());
        assert_eq!("reality/string.string@0.1.0", format!("{name:#}"));
        assert_eq!("reality/0.1.0/string/string", name.path().to_string_lossy());
        assert_eq!("reality/string.string", name.plugin_ref());
        assert_eq!("reality/string.string@0.1.0", name.full_plugin_ref());
        assert_eq!("reality/0.1.0/string/string", name.path().to_string_lossy());
    }
}
