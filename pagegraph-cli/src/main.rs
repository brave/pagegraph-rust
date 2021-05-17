//! CLI for pagegraph-rust

use pagegraph::from_xml::read_from_file;

use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("paghgraph-rust CLI")
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
                          .get_matches();

    let graph_file = matches.value_of("graph_file").unwrap();

    let graph = read_from_file(&graph_file);

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
    }
}
