//! Prints out all info from the graph about the given request ID.

use std::convert::TryFrom;

use pagegraph::from_xml::read_from_file;
use pagegraph::{graph::{Edge, FrameId, HasFrameId}, types::{EdgeType, NodeType, RequestType}};

/// Custom serializer for `RequestType`, so that `RequestInfo` can hold it directly rather than a
/// string representation.
fn serialize_request_type<S>(request_type: &RequestType, serializer: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer {
    serializer.serialize_str(request_type.as_str())
}

fn main() {
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

    let mut args = std::env::args().skip(1);
    let graph_file = args.next().expect("Provide a path to a `.graphml` file");
    let id_arg = args.next().expect("Provide a request id, optionally followed by a frame id").parse::<usize>().expect("Edge id should be parseable as a number");
    let frame_id = args.next().map(|frame_id_str| FrameId::try_from(frame_id_str.as_str()).expect("Frame id should be parseable"));

    let mut graph = read_from_file(&graph_file);

    graph.all_remote_frame_ids().into_iter().for_each(|remote_frame_id| {
        let mut frame_path = std::path::Path::new(&graph_file).to_path_buf();
        frame_path.set_file_name(format!("page_graph_{}.0.graphml", remote_frame_id));
        if !frame_path.exists() {
            // We have to just ignore the remote frame's contents if we couldn't successfully record any.
            return;
        }
        let frame_graph = read_from_file(frame_path.to_str().expect("failed to convert frame path to a string"));
        graph.merge_frame(frame_graph, &remote_frame_id);
    });

    let mut start_edge: Option<&Edge> = None;
    let mut complete_edge: Option<&Edge> = None;

    graph.edges.iter().for_each(|(edge_id, e)| {
        if edge_id.get_frame_id() != frame_id {
            return;
        }
        match &e.edge_type {
            EdgeType::RequestStart { request_id, .. } if *request_id == id_arg => {
                assert!(start_edge.is_none(), "multiple RequestStart edges for request id");
                start_edge = Some(e);
            }
            EdgeType::RequestComplete { request_id, .. } if *request_id == id_arg => {
                assert!(start_edge.is_none(), "multiple RequestComplete edges for request id");
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
