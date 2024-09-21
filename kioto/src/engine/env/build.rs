use super::{default_create_env, EngineConfig, Env};
use reality::{plugin::Name, Content, Repr, Resource, Uuid};
use std::{collections::BTreeMap, path::PathBuf, str::FromStr};
use tracing::{debug, error};

/// Type-alias for a function that creates an environment
type LoadEnv = fn(String, PathBuf) -> Env;

/// Struct containing tools for creating a new environment
///
/// The default implementation will automatically include all plugins implemented in this crate
pub struct Builder {
    /// Label for this environment
    label: String,
    /// Function for creating a new environment
    env_loader: LoadEnv,
}

impl Builder {
    /// Creates an new builder using the default env_loader
    /// 
    /// The default env_loader implementation will include all plugins from this crate
    #[inline]
    pub fn default_env(label: impl Into<String>) -> Self {
        Self::new(label, default_create_env)
    }

    /// Creates a new env builder
    #[inline]
    pub fn new(label: impl Into<String>, env_loader: LoadEnv) -> Self {
        Self {
            label: label.into().trim_matches(['"']).to_string(),
            env_loader,
        }
    }

    /// Tries to build an environment from files in a source root and,
    /// constructing the required folder structure to load from the target root
    pub fn build_env(
        &self,
        source_root: impl Into<PathBuf>,
        target_root: impl Into<PathBuf>,
    ) -> std::io::Result<()> {
        let source_root: PathBuf = source_root.into().join(&self.label);
        let target_root: PathBuf = target_root.into().join(&self.label);
        let dir_reader = source_root.read_dir()?;

        let mut copy_tasks = BTreeMap::<(Name, String), PathBuf>::new();
        let mut config = EngineConfig::default();
        for entry in dir_reader {
            match entry {
                Ok(ref entry) if entry.path().is_file() => {
                    match entry.path().extension().and_then(|p| p.to_str()) {
                        Some("toml") => {
                            let content = std::fs::read_to_string(entry.path())?;
                            match toml_edit::DocumentMut::from_str(&content) {
                                Ok(doc) => {
                                    if let Some(event_name) = entry
                                        .path()
                                        .file_stem()
                                        .and_then(|e| e.to_str())
                                        .map(|e| e.to_string())
                                    {
                                        match config.parse_build_document(&event_name, doc) {
                                            Ok(name) => {
                                                debug!(
                                                    path = entry.path().to_string_lossy().to_string(),
                                                    plugin = name.full_plugin_ref().to_string(),
                                                    "Built file"
                                                );

                                                if let Some(_replaced) = copy_tasks.insert((name, event_name), entry.path()) {
                                                    // TODO: Shouldn't be able to replace
                                                }
                                            }
                                            Err(err) => {
                                                error!(
                                                    "Could not process file {:?}, {err}",
                                                    entry.path()
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    error!("Skipping toml file {:?}", entry.path());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    error!("{e}");
                }
                _ => {}
            }
        }

        if copy_tasks.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "No valid files were found".to_string()));
        }

        match toml::to_string(&config) {
            Ok(config) => {
                std::fs::create_dir_all(&target_root)?;
                let config_path = target_root.join("config.toml");
                std::fs::write(config_path, config)?;
                for ((name, event_name), source) in copy_tasks {
                    let to_dir = target_root.join("etc").join(name.path());
                    std::fs::create_dir_all(&to_dir)?;
                    let to = to_dir.join(format!("{event_name}.toml"));
                    debug!("Copying {source:?} -> {to:?}");
                    std::fs::copy(&source, &to)?;
                }
                Ok(())
            },
            Err(err) => {
                Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))
            },
        }
    }

    /// Tries to initialize from some root directory,
    ///
    /// Will load all config immediately and set the env loader with the loaded config.
    ///
    /// The EnvLoader can then be used to load events from event configurations
    #[inline]
    pub fn load_env(&self, root: impl Into<PathBuf>) -> std::io::Result<Env> {
        let root = root.into();
        let mut config = EngineConfig::from_file_system(root.clone(), &self.label)?;
        let mut loader = (self.env_loader)(self.label.to_string(), root);
        config
            .load(&mut loader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{e:?}")))?;
        loader.config = config;
        loader.label = self.label.clone();
        Ok(loader)
    }
}

impl Resource for Builder {}
impl Repr for Builder {}
impl Content for Builder {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        let mut crc = reality::content::crc().digest();
        crc.update(self.label.as_bytes());
        Uuid::from_u64_pair(crc.finalize(), 0)
    }
}
