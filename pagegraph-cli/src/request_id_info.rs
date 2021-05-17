//! Prints out all info from the graph about the given request ID.

use pagegraph::{graph::{Edge, FrameId, HasFrameId, PageGraph}, types::{EdgeType, NodeType, RequestType}};

/// Custom serializer for `RequestType`, so that `RequestInfo` can hold it directly rather than a
/// string representation.
fn serialize_request_type<S>(request_type: &RequestType, serializer: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer {
    serializer.serialize_str(request_type.as_str())
}

pub fn main(graph: &PageGraph, request_id_arg: usize, frame_id: Option<FrameId>) {
    #[derive(serde::Serialize)]
    struct RequestInfo {
        // RequestStart
        #[serde(serialize_with = "serialize_request_type")]
        request_type: RequestType,
        //status: String,
        //request_id: usize,

        // Resource
        url: String,

        // RequestComplete
        resource_type: String,
        status: String,
        value: Option<String>,
        response_hash: Option<String>,
        //request_id: usize,
        headers: String,
        size: String,
    }

    let mut start_edge: Option<&Edge> = None;
    let mut complete_edge: Option<&Edge> = None;

    graph.edges.iter().for_each(|(edge_id, e)| {
        if edge_id.get_frame_id() != frame_id {
            return;
        }
        // There can be multiple request start and complete edges for the same request id, if they
        // represent requests to the same cached resource. However, the information retrieved here
        // should be identical, so we can use any matching edge.
        match &e.edge_type {
            EdgeType::RequestStart { request_id, .. } if *request_id == request_id_arg => {
                start_edge = Some(e);
            }
            EdgeType::RequestComplete { request_id, .. } if *request_id == request_id_arg => {
                complete_edge = Some(e);
            }
            _ => (),
        }
    });

    let start_edge = start_edge.expect("No RequestStart edge for request id");
    let complete_edge = complete_edge.expect("No RequestComplete edge for request id");

    let start_target = graph.target_node(start_edge);
    let complete_source = graph.source_node(complete_edge);

    assert_eq!(start_target.id, complete_source.id, "RequestStart and RequestComplete do not refer to the same Resource");

    let request_info = if let EdgeType::RequestComplete { resource_type, status, value, response_hash, headers, size, .. } = &complete_edge.edge_type {
        if let EdgeType::RequestStart { request_type, .. } = &start_edge.edge_type {
            if let NodeType::Resource { url } = &start_target.node_type {
                RequestInfo {
                    request_type: request_type.clone(),

                    url: url.clone(),

                    resource_type: resource_type.clone(),
                    status: status.clone(),
                    value: value.clone(),
                    response_hash: response_hash.clone(),
                    headers: headers.clone(),
                    size: size.clone(),
                }
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    println!("{}", serde_json::to_string(&request_info).unwrap());
}
