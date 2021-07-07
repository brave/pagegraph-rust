//! Given an adblock network rule, prints out the nodes for resources that match that rule.

use pagegraph::graph::PageGraph;

pub fn main(graph: &PageGraph, filter_rules: Vec<String>) {
    let matching_elements = graph.resources_matching_filters(graph, filter_rules);
    println!("{}", serde_json::to_string(&matching_elements).unwrap())
}
