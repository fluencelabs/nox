use futures::FutureExt;
use particle_builtins::{ok, CustomService, NodeInfo};
use particle_execution::ServiceFunction;
use serde_json::json;

pub fn make_peer_builtin(node_info: NodeInfo) -> (String, CustomService) {
    (
        "peer".to_string(),
        CustomService::new(
            vec![("identify", make_peer_identify_closure(node_info))],
            None,
        ),
    )
}
fn make_peer_identify_closure(node_info: NodeInfo) -> ServiceFunction {
    ServiceFunction::Immut(Box::new(move |_args, _params| {
        let node_info = node_info.clone();
        async move { ok(json!(node_info)) }.boxed()
    }))
}
