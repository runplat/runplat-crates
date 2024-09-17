use reality::{Resource, Plugin};
use serde::Serialize;
use url::Url;

/// Plugin to execute a request
#[derive(Serialize)]
pub struct Request {
    url: Url,
}

impl Plugin for Request {
    fn call(_: reality::plugin::Bind<Self>) -> reality::Result<reality::plugin::SpawnWork> {
        todo!()
    }

    fn package() -> (&'static str, &'static str) {
        (env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
    }
}

impl Resource for Request {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_plugin() {
        let name = Request::name();
        assert_eq!("kioto/plugins.request", name.plugin_ref());
    }
}