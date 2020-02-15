use crate::graph::{ PageGraph, Edge, EdgeId, Node, NodeId };
use crate::types::{ EdgeType, NodeType };

impl PageGraph {
    pub fn filter_edges<F: Fn(&EdgeType) -> bool>(&self, f: F) -> Vec<(&EdgeId, &Edge)> {
        self.edges.iter().filter(|(_id, edge)| {
            f(&edge.edge_type)
        }).collect()
    }

    pub fn filter_nodes<F: Fn(&NodeType) -> bool>(&self, f: F) -> Vec<(&NodeId, &Node)> {
        self.nodes.iter().filter(|(_id, node)| {
            f(&node.node_type)
        }).collect()
    }
}
