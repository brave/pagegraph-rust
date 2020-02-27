use crate::graph::{ PageGraph, Edge, EdgeId, Node, NodeId };
use crate::types::{ EdgeType, NodeType };

use petgraph::Direction;

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

    /// Returns a sorted Vec including 1 edge representing every time the given HtmlElement node was
    /// modified in the page.
    pub fn all_html_element_modifications(&self, node_id: NodeId) -> Vec<(&EdgeId, &Edge)> {
        let element = self.nodes.get(&node_id).unwrap();

        if let NodeType::HtmlElement { node_id: _html_node_id, .. } = element.node_type {
            let mut modifications: Vec<_> = self.graph.neighbors_directed(node_id, Direction::Incoming).map(|node| {
                let edge_id = self.graph.edge_weight(node, node_id).unwrap();
                (edge_id, self.edges.get(edge_id).unwrap())
            }).filter(|(_id, edge)| {
                match edge.edge_type {
                    EdgeType::Structure { .. } => false,
                    _ => true,
                }
            }).collect();

            modifications.sort_by_key(|(_, edge)| edge.edge_timestamp.expect("HTML element modification had no timestamp"));

            modifications
        } else {
            panic!("Supply a node with HtmlElement node type");
        }
    }
}
