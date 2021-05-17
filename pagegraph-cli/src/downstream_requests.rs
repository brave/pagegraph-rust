//! Prints out all downstream network requests of a given edge from the graph.

use pagegraph::{graph::{EdgeId, PageGraph}, types::EdgeType};

pub fn main(graph: &PageGraph, edge_id: EdgeId) {

    let downstream_effects = graph.all_downstream_effects_of(graph.edges.get(&edge_id).unwrap()).into_iter().filter_map(|edge| {
        if let EdgeType::RequestStart { request_id, .. } = edge.edge_type {
            Some((request_id, format!("{}", edge.id)))
        } else {
            None
        }
    }).collect::<Vec<_>>();

    println!("{}", serde_json::to_string(&downstream_effects).unwrap());
}
