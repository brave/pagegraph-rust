use rayon::prelude::*;

use pagegraph::from_xml::read_from_file;
use pagegraph::types::{NodeType, EdgeType};

fn main() {
    let graph_files = std::env::args().skip(1).collect::<Vec<String>>();
    if graph_files.len() == 0 {
        eprintln!("usage: {0} GRAPHML_FILE [GRAPHML_FILE ...]", std::env::args().take(1).next().expect("no argv[0]?!"));
    }
    
    graph_files.into_par_iter().for_each(|graph_file| {
        let graph = read_from_file(&graph_file);
        
        let total_nodes = graph.nodes.len();
        let total_edges = graph.edges.len();
        let mut total_dom_nodes = 0;
        let mut net_dom_nodes = 0;
        let mut touched_dom_nodes = 0;
        graph.nodes.iter().for_each(|(id, node)| {
            match node.node_type {
                NodeType::HtmlElement { is_deleted, .. } => {
                    total_dom_nodes += 1;
                    if !is_deleted { net_dom_nodes += 1; }
                    let html_mods = graph.all_html_element_modifications(*id);
                    if html_mods.len() > 2 { touched_dom_nodes += 1; }
                }
                _ => {}
            }
        });

        let completed_requests = graph.filter_edges(|edge_type| {
            match edge_type {
                EdgeType::RequestComplete { .. } => true,
                _ => false,
            }
        }).len();
        let event_listenings = graph.filter_edges(|edge_type| {
            match edge_type {
                EdgeType::AddEventListener { .. } => true,
                _ => false,
            }
        }).len();
        

        println!("{0}: {1} nodes, {2} edges, {3}/{4}/{5} DOM nodes created/retained/touched, {6} completed requests, {7} event registrations",
            graph_file,
            total_nodes,
            total_edges,
            total_dom_nodes,
            net_dom_nodes,
            touched_dom_nodes,
            completed_requests,
            event_listenings,
        );
    });
}
