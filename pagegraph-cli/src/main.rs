//! CLI for pagegraph-rust

use pagegraph::from_xml::read_from_file;
use pagegraph::graph::{EdgeId, FrameId};

use clap::{App, Arg, SubCommand};

mod adblock_rules;
mod request_id_info;
mod downstream_requests;

fn main() {
    let matches = App::new("pagegraph-rust CLI")
        .version("1.0")
        .arg(Arg::with_name("graph_file")
            .short("f")
            .value_name("FILE")
            .help("Set the graph to query")
            .takes_value(true)
            .required(true))
        .subcommand(SubCommand::with_name("identify")
            .about("Check information about a particular node or edge id in the graph")
            .arg(Arg::with_name("id")
                .help("Node or edge id")
                .takes_value(true)
                .required(true)))
        .subcommand(SubCommand::with_name("adblock_rules")
            .about("Find network requests matching a given adblock rule")
            .arg(Arg::with_name("filter_rule")
                .help("Adblock rule to use, using ABP syntax")
                .takes_value(true)
                .value_name("RULE")
                .required(true)))
        .subcommand(SubCommand::with_name("downstream_requests")
            .about("Find network requests initiated as a result of a given edge in the graph")
            .arg(Arg::with_name("edge_id")
                .help("Edge id to check downstream requests for")
                .takes_value(true)
                .value_name("ID")
                .required(true)))
        .subcommand(SubCommand::with_name("request_id_info")
            .about("Get all information from the graph associated with a particular Blink request id")
            .arg(Arg::with_name("request_id")
                .help("Blink request id from the graph")
                .takes_value(true)
                .value_name("REQUEST")
                .required(true))
            .arg(Arg::with_name("frame_id")
                .help("Optional frame id that the request id is associated with, defaults to the root frame")
                .takes_value(true)
                .value_name("FRAME")
                .required(false)))
        .get_matches();

    let graph_file = matches.value_of("graph_file").unwrap();

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

    if let Some(matches) = matches.subcommand_matches("identify") {
        let id = matches.value_of("id").unwrap().parse::<usize>().expect("Could not parse id as a number");

        if let Some(node) = graph.nodes.get(&pagegraph::graph::NodeId::from(id)) {
            println!("Node n{}", id);
            println!("Timestamp: {}", node.node_timestamp);
            println!("Type: {:?}", node.node_type);

            println!("");
            println!("Incoming edges");
            graph.incoming_edges(node).for_each(|edge| {
                println!("  {:?}", edge.id);
                println!("    Timestamp: {:?}", edge.edge_timestamp);
                println!("    Type: {:?}", edge.edge_type);
            });

            println!("");
            println!("Outgoing edges");
            graph.outgoing_edges(node).for_each(|edge| {
                println!("  {:?}", edge.id);
                println!("    Timestamp: {:?}", edge.edge_timestamp);
                println!("    Type: {:?}", edge.edge_type);
            });
        } else if let Some(edge) = graph.edges.get(&pagegraph::graph::EdgeId::from(id)) {
            println!("Edge e{}", id);
            println!("Timestamp: {:?}", edge.edge_timestamp);
            println!("Type: {:?}", edge.edge_type);

            println!("");
            println!("Source node");
            let source_node = graph.source_node(edge);
            println!("  {:?}", source_node.id);
            println!("    Timestamp: {:?}", source_node.node_timestamp);
            println!("    Type: {:?}", source_node.node_type);

            println!("");
            println!("Target node");
            let target_node = graph.target_node(edge);
            println!("  {:?}", target_node.id);
            println!("    Timestamp: {:?}", target_node.node_timestamp);
            println!("    Type: {:?}", target_node.node_type);
        } else {
            println!("No node or edge with id {} was found in this graph.", id);
        }
    } else if let Some(matches) = matches.subcommand_matches("adblock_rules") {
        let rule = matches.value_of("filter_rule").unwrap();
        adblock_rules::main(&graph, rule);
    } else if let Some(matches) = matches.subcommand_matches("downstream_requests") {
        use std::convert::TryFrom;
        let edge_id = EdgeId::try_from(matches.value_of("edge_id").unwrap()).expect("Provided edge id was invalid");
        downstream_requests::main(&graph, edge_id);
    } else if let Some(matches) = matches.subcommand_matches("request_id_info") {
        use std::convert::TryFrom;
        let request_id = matches.value_of("request_id").unwrap().parse::<usize>().expect("Request id should be parseable as a number");
        let frame_id: Option<FrameId> = matches.value_of("frame_id").map(|frame_id_str| FrameId::try_from(frame_id_str).expect("Frame id should be parseable"));
        request_id_info::main(&graph, request_id, frame_id);
    }
}
