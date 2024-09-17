use reality::{Plugin, Resource, Version};
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

    fn version() -> Version {
        env!("CARGO_PKG_VERSION").parse().expect("should be a version because cargo will complain first")
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