use std::path::PathBuf;
use runir::*;
use semver::Version;

/// Struct containing name data
#[derive(Hash, Clone)]
pub struct Name {
    package: String,
    version: Version,
    module: String,
    plugin: String,
    path: PathBuf,
}

impl Name {
    /// Creates a new name and generates a path to reference the type with
    /// 
    /// ## Path format
    /// 
    /// The format of the path is `{package-name}/{package-version}/{upper-most-module}/{type-name}`
    pub fn new<T>() -> Name
    where
        T: ?Sized,
    {
        let mut name = Name {
            package: env!("CARGO_PKG_NAME").to_lowercase(),
            version: env!("CARGO_PKG_VERSION")
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

    /// Returns this name as a path format
    #[inline]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Repr for Name {}
impl Resource for Name {}
