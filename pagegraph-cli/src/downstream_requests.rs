//! Prints out all downstream network requests of a given edge from the graph.

use pagegraph::{graph::{EdgeId, PageGraph}, types::EdgeType};
use pagegraph::graph::DownstreamRequests;
use pagegraph::types::NodeType;

pub fn main(graph: &PageGraph, edge_id: EdgeId) {
    let edge = graph.edges.get(&edge_id).unwrap();
    let all_downstream_requests = graph
        .all_downstream_requests_nested(graph.edges.get(&edge_id).unwrap());
    let node = graph.target_node(edge);
    let url = match &node.node_type {
        NodeType::Resource { url } => url,
        _ => unreachable!()
    };
    match &edge.edge_type {
        EdgeType::RequestStart {request_id, request_type, ..} => {
            let top_level = DownstreamRequests {
                request_id: *request_id,
                url: url.to_string(),
                request_type: request_type.clone(),
                node_id: node.id,
                children: all_downstream_requests
            };
            println!("{}", serde_json::to_string(&top_level).unwrap());
        },
        _ => panic!("Edge is not a RequestStart!")
    };
}
