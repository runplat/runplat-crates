mod plugin;

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    PluginAborted,
    TokioJoinError
}

impl From<tokio::task::JoinError> for Error {
    fn from(_: tokio::task::JoinError) -> Self {
        Self::TokioJoinError
    }
}

mod tests {
    use crate::plugin::{Namespace, Plugin};

    struct TestPlugin;

    impl Plugin for TestPlugin {
        fn namespace() -> crate::plugin::Namespace {
            Namespace::default()
        }
    
        fn call(context: crate::plugin::Context) -> Option<crate::plugin::AsyncContext> {
            let tc = context.clone();
            Some(context.spawn(|_| async {
                let item = tc.store().item(0).cloned();

                Ok(tc)
            }))
        }
    }
}