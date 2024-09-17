use reality::{BincodeContent, Content, Plugin, Resource, Uuid, Version};
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

impl Content for Request {
    fn state_uuid(&self) -> Uuid {
        BincodeContent::new(self).unwrap().state_uuid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_plugin() {
        let name = Request::name();
        assert_eq!("kioto/plugins.request", name.plugin_ref());
    }
}