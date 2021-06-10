//! Given an adblock network rule, prints out the nodes for resources that match that rule.

use pagegraph::graph::PageGraph;
use pagegraph::types::{EdgeType, NodeType};
use std::fs::File;
use std::io::{BufReader, BufRead};

pub fn main(graph: &PageGraph, filter_rule: Option<&str>, filter_list_path: Option<&str>) {
    // Check which one is defined
    let filter_rules = if let Some(rule) = filter_rule {
        vec![rule.to_string()]
    } else {
        // open file
        let file = File::open(filter_list_path
            .expect("At least one of path_to_filterlist or filter_rule must be defined")).unwrap();
        let reader = BufReader::new(file);
        let rules: Vec<_> = reader.lines()
            .map(|l| l.expect("Could not parse line"))
            .collect();
        rules
    };
    let mut matching_elements = vec![];
    for filter_rule in &filter_rules {
        matching_elements.extend(graph.resources_matching_filter(&filter_rule));
    }

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
