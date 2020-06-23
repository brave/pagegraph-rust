//! Prints out all URLs from a page that match the given network filter adblock rule.

use pagegraph::from_xml::read_from_file;
use pagegraph::types::NodeType;

fn main() {
    let mut args = std::env::args().skip(1);
    let graph_file = args.next().expect("Provide a path to a `.graphml` file");
    let filter_rule = args.next().expect("Provide a network filter rule");

    let graph = read_from_file(&graph_file);

    let matching_elements = graph.resources_matching_filter(&filter_rule);

    matching_elements.iter().for_each(|(_id, matching_element)| {
        if let NodeType::Resource { url, .. } = &matching_element.node_type {
            println!("Matched {}", url);
        }
    });
}
