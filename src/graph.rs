use std::collections::HashMap;

use petgraph::graphmap::DiGraphMap;

use crate::types::{ NodeType, EdgeType };

/// Contains metadata summary extracted from the <desc> block of a PageGraph GraphML file
#[derive(Debug)]
pub struct PageGraphMeta {
    pub version: String,
    pub url: Option<String>,
    pub is_root: Option<bool>,
}

/// The main PageGraph data structure.
#[derive(Debug)]
pub struct PageGraph {
    pub meta: Option<PageGraphMeta>,
    pub edges: HashMap<EdgeId, Edge>,
    pub nodes: HashMap<NodeId, Node>,
    pub graph: DiGraphMap<NodeId, Vec<EdgeId>>,
}

/// An identifier used to reference a node.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct NodeId(usize);

impl From<usize> for NodeId {
    fn from(v: usize) -> Self {
        NodeId(v)
    }
}

/// A node, representing a side effect of a page load.
#[derive(Debug)]
pub struct Node {
    pub node_timestamp: isize,
    pub node_type: NodeType,
}

/// An identifier used to reference an edge.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct EdgeId(usize);

impl From<usize> for EdgeId {
    fn from(v: usize) -> Self {
        EdgeId(v)
    }
}

/// An edge, representing an action taken during page load.
#[derive(Debug)]
pub struct Edge {
    pub edge_timestamp: Option<isize>,
    pub edge_type: EdgeType,
}
