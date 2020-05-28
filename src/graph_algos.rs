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

    /// Get a collection of all Resource nodes whose requests were intiated by a given Script node or HtmlElement node with tag_name "script".
    ///
    /// For script nodes, associated resources are directly attached by a Request Start edge.
    /// For script HTML element nodes, associated resources are either directly attached by a
    /// Request Start edge (`src="..."`), or additionally by a Request Start edge on an attached
    /// Script node.
    pub fn resources_from_script(&self, node_id: NodeId) -> Vec<(NodeId, &Node)> {
        let element = self.nodes.get(&node_id).unwrap();

        let mut resulting_resources: Vec<NodeId> = self
            .graph.neighbors_directed(node_id, Direction::Outgoing)
            .filter(|neighbor_id| match self.nodes.get(&neighbor_id).unwrap().node_type { NodeType::Resource { .. } => true, _ => false })
            .collect();

        match element.node_type {
            NodeType::Script { .. } => (),
            NodeType::HtmlElement { ref tag_name, .. } if tag_name == "script" => {
                let mut resources_from_executed_script: Vec<NodeId> = self
                    .graph.neighbors_directed(node_id, Direction::Outgoing)
                    .filter(|neighbor_id| match self.nodes.get(&neighbor_id).unwrap().node_type { NodeType::Script { .. } => true, _ => false })
                    .map(|attached_script_id| self.graph.neighbors_directed(attached_script_id, Direction::Outgoing))
                    .flatten()
                    .filter(|attached_script_neighbor_id| match self.nodes.get(attached_script_neighbor_id).unwrap().node_type { NodeType::Resource { .. } => true, _ => false })
                    .collect();
                resulting_resources.append(&mut resources_from_executed_script);
            },
            _ => panic!("Supply a node with Script node type, or an HtmlElement node with tag_name \"script\""),
        }

        resulting_resources.into_iter().map(|node_id| (node_id, self.nodes.get(&node_id).unwrap())).collect()
    }
}
