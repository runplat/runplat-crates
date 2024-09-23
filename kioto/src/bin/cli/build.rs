use std::path::PathBuf;

use clap::Args;

/// Build arguments
#[derive(Args)]
pub struct BuildArgs {
    /// Label of the environment to build
    #[clap(long, short)]
    label: Option<String>,
    /// Target directory to output the build to
    #[clap(long, short)]
    output: Option<PathBuf>,
    /// Source context
    source: String,
}

impl BuildArgs {
    pub fn build(self) -> BuildExec {
        let label = self.label.unwrap_or(String::from("default"));
        BuildExec {
            label,
            target_dir: self.output.unwrap_or(PathBuf::from(".kt")),
            source: SourceContext::Dir(PathBuf::from(self.source)),
        }
    }
}

pub struct BuildExec {
    label: String,
    target_dir: PathBuf,
    source: SourceContext,
}

enum SourceContext {
    Dir(PathBuf)
}

impl SourceContext {
    pub fn source_path(&self) -> &PathBuf {
        match self {
            SourceContext::Dir(path_buf) => &path_buf,
        }
    }
}

impl BuildExec {
    pub fn exec(self) {
        let builder = kioto::engine::EnvBuilder::default_env(&self.label);
        builder
            .build_env(self.source.source_path(), self.target_dir)
            .unwrap();
    }
}
