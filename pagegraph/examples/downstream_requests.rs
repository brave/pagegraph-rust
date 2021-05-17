//! Prints out all downstream network requests of a given edge from the graph.

use std::convert::TryFrom;

use pagegraph::from_xml::read_from_file;
use pagegraph::{graph::EdgeId, types::EdgeType};

fn main() {
    let mut args = std::env::args().skip(1);
    let graph_file = args.next().expect("Provide a path to a `.graphml` file");
    let edge_id = EdgeId::try_from(args.next().expect("Provide an edge id").as_str()).expect("Provided edge id was invalid");

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

    let downstream_effects = graph.all_downstream_effects_of(graph.edges.get(&edge_id).unwrap()).into_iter().filter_map(|edge| {
        if let EdgeType::RequestStart { request_id, .. } = edge.edge_type {
            Some((request_id, format!("{}", edge.id)))
        } else {
            None
        }
    }).collect::<Vec<_>>();

    println!("{}", serde_json::to_string(&downstream_effects).unwrap());
}
