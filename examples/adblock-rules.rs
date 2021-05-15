//! Given an adblock network rule, prints out the nodes for resources that match that rule.

use pagegraph::from_xml::read_from_file;
use pagegraph::types::{EdgeType, NodeType};

fn main() {
    let mut args = std::env::args().skip(1);
    let graph_file = args.next().expect("Provide a path to a `.graphml` file");
    let filter_rule = args.next().expect("Provide a network filter rule");

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

    let matching_elements = graph.resources_matching_filter(&filter_rule);

    #[derive(serde::Serialize)]
    struct MatchingResource {
        url: String,
        node_id: String,
        request_types: Vec<String>,
        requests: Vec<(usize, String)>,
    }

    let matching_resources = matching_elements.iter().filter(|(_id, matching_element)| {
        matches!(&matching_element.node_type, NodeType::Resource { .. })
    }).map(|(node_id, resource_node)| {
        let requests = graph.incoming_edges(resource_node).filter_map(|edge| if let EdgeType::RequestStart { request_id, .. } = &edge.edge_type {
            Some((*request_id, format!("{}", edge.id)))
        } else {
            None
        }).collect::<Vec<_>>();

        let request_types = graph.resource_request_types(node_id).into_iter().map(|(ty, _)| ty).collect();

        if let NodeType::Resource { url } = &resource_node.node_type {
            MatchingResource {
                url: url.clone(),
                node_id: format!("{}", node_id),
                request_types,
                requests,
            }
        } else { unreachable!() }
    }).collect::<Vec<_>>();

    println!("{}", serde_json::to_string(&matching_resources).unwrap())
}
