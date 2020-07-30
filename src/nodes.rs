use std::collections::HashMap;

use petgraph::Direction;

use crate::graph::{Edge, EdgeId, Node, NodeId, PageGraph};
use crate::graph_algos::NodeRef;
use crate::types::{EdgeType, NodeType};

impl PageGraph {
  pub fn creator_of_html_node(&self, node_ref: &NodeRef) -> NodeRef {
      let (created_node_id, created_node) = node_ref;
      assert!(match self.nodes.get(created_node_id).unwrap().node_type {
          NodeType::HtmlElement { .. } | NodeType::TextNode { .. } => true,
          _ => {
            eprintln!("Invalid node, cannot determine creator of {}", created_node.node_type);
            false
          }
      });

      let creating_node_ref = self.graph
          .edges_directed(*created_node_id, Direction::Incoming)
          .find_map(
              |(from_node_id, _, edge_id)| match self.edges.get(&edge_id).unwrap().edge_type {
                  EdgeType::CreateNode { .. } => {
                      Some((from_node_id, self.nodes.get(&from_node_id).unwrap()))
                  }
                  _ => None,
              },
          )
          .unwrap();

      assert!(match created_node.node_type {
          NodeType::Script { .. } | NodeType::Parser { .. } => true,
          _ => {
            let creating_node = creating_node_ref.1;
            eprintln!("Found unexpected node {} creating {}", creating_node.node_type, creating_node.node_type);
            false
          }
      });

      creating_node_ref
  }

  pub fn final_markup_of_node(&self, node_ref: NodeRef) -> String {
    let (node_id, &node) = node_ref;
    let html_tag_name = match node.node_type {
      NodeType:::HtmlElement { tag_name: tag_name, .. } => tag_name,
      _ => panic!("Tried to generate HTML markup from invalid node type: {}", node.node_type),
    };
    let html_attrs: HashMap<String, String> = HashMap::new();
  }
}

pub fn html_element_owning_script<'a>(pg: &'a PageGraph, node_ref: &'a NodeRef) -> NodeRef<'a> {}
