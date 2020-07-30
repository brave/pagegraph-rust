#[allow(dead_code)]
extern crate adblock;

use std::vec::Vec;

use adblock::filters::network::NetworkFilter;

use crate::graph::{Node, NodeId, PageGraph};
use crate::types::NodeType;

pub type Url = String;

pub enum StorageEndpoint {
    Cookie,
    LocalStorage,
    SessionStorage,
}

pub enum JSApiAction {
    Call,
    Read,
}

pub struct ResponsibleScript {
    cause: Option<Box<ResponsibleScript>>,
    url: Option<Url>,
    frame_stack: Vec<Url>,
}

pub struct JSApiCall {
    endpoint: String,
    action: JSApiAction,
    args: Option<Vec<String>>,
}

pub enum QueryResultMatch {
    IncludedLeafScript(ResponsibleScript),
    JSApiCall,
}

pub type QueryResult = Vec<QueryResultMatch>;

pub fn caused_storage(
    graph: &PageGraph,
    filter: &Option<NetworkFilter>,
    verbose: bool,
) -> QueryResult {
    let script_node_refs = match filter {
        Some(f) => graph.resources_matching_filter(&f.to_string()),
        None => graph.filter_nodes(|nt| match nt {
            NodeType::Script { .. } => true,
            _ => false,
        }),
    };

    if verbose {
        println!("{} scripts matched conditions.", script_node_refs.len())
    }

    script_node_refs.iter().map(|(node_id, node_ref)| {});

    Vec::new()
}
