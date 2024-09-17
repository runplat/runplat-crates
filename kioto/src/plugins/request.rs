use reality::{runir::Resource, Plugin};
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
}

impl Resource for Request {}
