//! CLI for pagegraph-rust

use pagegraph::from_xml::read_from_file;
use pagegraph::graph::{EdgeId, FrameId};

use clap::{App, Arg, SubCommand};
use std::fs::File;
use std::io::{BufReader, BufRead};

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
                .short("r")
                .long("rule")
                .takes_value(true)
                .required_unless("path_to_filterlist"))
            .arg(Arg::with_name("path_to_filterlist")
                .short("l")
                .long("list")
                .required_unless("filter_rule")
                .help("Set path to filterlist file (newline-separated adblock rules) to use")
                .takes_value(true))
            .arg(Arg::with_name("match_only_exceptions")
                .short("e")
                .long("only-exceptions")
                .help("Only match on exception rules")
                .takes_value(false)
                .required(false)))
        .subcommand(SubCommand::with_name("downstream_requests")
            .about("Find network requests initiated as a result of a given edge in the graph")
            .arg(Arg::with_name("requests")
                .help("Get just the list of downstream resource IDs")
                .takes_value(false)
                .short("r")
                .long("requests")
                .required(false))
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
            .arg(Arg::with_name("source")
                .help("Print just the escaped source")
                .takes_value(false)
                .short("s")
                .long("source")
                .required(false))
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
        let rule = matches.value_of("filter_rule");
        let filterlist = matches.value_of("path_to_filterlist");
        let only_exceptions = matches.is_present("match_only_exceptions");
        let filter_rules = if let Some(rule) = rule {
            vec![rule.to_string()]
        } else {
            // open file
            let file = File::open(filterlist
                .expect("At least one of path_to_filterlist or filter_rule must be defined")).unwrap();
            let reader = BufReader::new(file);
            let rules: Vec<_> = reader.lines()
                .map(|l| l.expect("Could not parse line"))
                .collect();
            rules
        };
        adblock_rules::main(&graph, filter_rules, only_exceptions);
    } else if let Some(matches) = matches.subcommand_matches("downstream_requests") {
        use std::convert::TryFrom;
        let just_requests = matches.is_present("requests");
        let edge_id = EdgeId::try_from(matches.value_of("edge_id").unwrap()).expect("Provided edge id was invalid");
        downstream_requests::main(&graph, edge_id, just_requests);
    } else if let Some(matches) = matches.subcommand_matches("request_id_info") {
        use std::convert::TryFrom;
        let request_id = matches.value_of("request_id").unwrap().parse::<usize>().expect("Request id should be parseable as a number");
        let just_source = matches.is_present("source");
        let frame_id: Option<FrameId> = matches.value_of("frame_id").map(|frame_id_str| FrameId::try_from(frame_id_str).expect("Frame id should be parseable"));
        request_id_info::main(&graph, request_id, frame_id, just_source);
    }
}
