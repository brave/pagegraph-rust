use std::collections::HashMap;
use std::convert::TryFrom;

use petgraph::graphmap::DiGraphMap;

use crate::types::{NodeType, EdgeType, RequestType};

#[derive(Debug)]
pub struct PageGraphDescriptor {
    pub version: String,
    pub about: String,
    pub url: String,
    pub is_root: bool,
    pub frame_id: FrameId,
    pub time: PageGraphTime,
}

#[derive(Debug)]
pub struct PageGraphTime {
    pub start: u64,
    pub end: u64,
}

/// The main PageGraph data structure.
#[derive(Debug)]
pub struct PageGraph {
    pub desc: PageGraphDescriptor,
    pub edges: HashMap<EdgeId, Edge>,
    pub nodes: HashMap<NodeId, Node>,
    pub graph: DiGraphMap<NodeId, Vec<EdgeId>>,

    next_node_id: std::cell::RefCell<usize>,
    next_edge_id: std::cell::RefCell<usize>,
}

impl PageGraph {
    pub fn new(desc: PageGraphDescriptor, edges: HashMap<EdgeId, Edge>, nodes: HashMap<NodeId, Node>, graph: DiGraphMap<NodeId, Vec<EdgeId>>) -> Self {
        Self {
            desc,
            edges,
            nodes,
            graph,
            next_edge_id: std::cell::RefCell::new(usize::MAX),
            next_node_id: std::cell::RefCell::new(usize::MAX),
        }
    }

    /// Returns a new edge id that is guaranteed not to collide with an existing id in the graph.
    pub(crate) fn new_edge_id(&self) -> EdgeId {
        let new_id = EdgeId::from(self.next_edge_id.replace_with(|id| *id - 1));
        assert!(!self.edges.contains_key(&new_id));
        new_id
    }

    pub fn source_node<'a>(&'a self, edge: &Edge) -> &'a Node {
        self.nodes.get(&edge.source).unwrap_or_else(|| panic!("Source node for edge {:?} could not be found in the graph", edge))
    }

    pub fn target_node<'a>(&'a self, edge: &Edge) -> &'a Node {
        self.nodes.get(&edge.target).unwrap_or_else(|| panic!("Target node for edge {:?} could not be found in the graph", edge))
    }

    pub fn outgoing_edges<'a>(&'a self, node: &Node) -> impl Iterator<Item=&'a Edge> {
        self.edges_iter_directed(node, petgraph::Direction::Outgoing)
    }

    pub fn incoming_edges<'a>(&'a self, node: &Node) -> impl Iterator<Item=&'a Edge> {
        self.edges_iter_directed(node, petgraph::Direction::Incoming)
    }

    fn edges_iter_directed<'a>(&'a self, node: &Node, direction: petgraph::Direction) -> impl Iterator<Item=&'a Edge> {
        self.graph.edges_directed(node.id, direction).map(move |(_a, _b, edge_ids)| {
            edge_ids
        })
            .flatten()
            .map(move |edge_id| {
                self.edges.get(&edge_id).unwrap()
            })
    }

    pub fn outgoing_neighbors<'a>(&'a self, node: &Node) -> impl Iterator<Item=&'a Node> {
        self.nodes_iter_directed(node, petgraph::Direction::Outgoing)
    }

    pub fn incoming_neighbors<'a>(&'a self, node: &Node) -> impl Iterator<Item=&'a Node> {
        self.nodes_iter_directed(node, petgraph::Direction::Incoming)
    }

    fn nodes_iter_directed<'a>(&'a self, node: &Node, direction: petgraph::Direction) -> impl Iterator<Item=&'a Node> {
        self.graph.neighbors_directed(node.id, direction).map(move |node_id| {
            self.nodes.get(&node_id).unwrap()
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, serde::Serialize)]
struct GraphItemId {
    id: usize,
    frame_id: Option<FrameId>,
}

impl From<usize> for GraphItemId {
    fn from(v: usize) -> Self {
        Self {
            id: v,
            frame_id: None
        }
    }
}

impl TryFrom<&str> for GraphItemId {
    type Error = ParseIdError;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        if let Some((id, frame_id)) = v.split_once(':') {
            let id = id.parse::<usize>()?;
            Ok(GraphItemId {
                id,
                frame_id: Some(FrameId::try_from(frame_id)?),
            })
        } else {
            let id = v.parse::<usize>()?;
            Ok(Self::from(id))
        }
    }
}

impl GraphItemId {
    fn copy_for_frame_id(&self, frame_id: &FrameId) -> Self {
        Self {
            id: self.id,
            frame_id: Some(frame_id.clone()),
        }
    }
}

pub trait HasFrameId {
    fn get_frame_id(&self) -> Option<FrameId>;
}

pub fn is_same_frame_context<A: HasFrameId, B: HasFrameId>(a: A, b: B) -> bool {
    a.get_frame_id() == b.get_frame_id()
}

/// An identifier used to reference a node.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, serde::Serialize)]
pub struct NodeId(GraphItemId);

impl From<usize> for NodeId {
    fn from(v: usize) -> Self {
        Self(v.into())
    }
}

impl NodeId {
    pub fn copy_for_frame_id(&self, frame_id: &FrameId) -> Self {
        Self(self.0.copy_for_frame_id(frame_id))
    }
}

impl HasFrameId for NodeId {
    fn get_frame_id(&self) -> Option<FrameId> {
        self.0.frame_id
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(frame_id) = self.get_frame_id() {
            write!(f, "n{}:{}", self.0.id, frame_id)
        } else {
            write!(f, "n{}", self.0.id)
        }
    }
}

impl TryFrom<&str> for NodeId {
    type Error = ParseIdError;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        if let Some(("", rest)) = v.split_once('n') {
            Ok(Self(GraphItemId::try_from(rest)?))
        } else {
            Err(ParseIdError::MissingPrefix)
        }
    }
}

/// Downstream requests tree
#[derive(serde::Serialize)]
pub struct DownstreamRequests {
    pub request_id: usize,
    pub url: String,
    pub request_type: RequestType,
    pub node_id: NodeId,
    pub children: Vec<DownstreamRequests>,
}

/// A node, representing a side effect of a page load.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub node_timestamp: isize,
    pub node_type: NodeType,
}

/// An identifier used to reference an edge.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, serde::Serialize)]
pub struct EdgeId(GraphItemId);

impl From<usize> for EdgeId {
    fn from(v: usize) -> Self {
        EdgeId(v.into())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseIdError {
    MissingPrefix,
    ParseIntError,
    FrameIdLength,
}

impl From<std::num::ParseIntError> for ParseIdError {
    fn from(_: std::num::ParseIntError) -> Self {
        Self::ParseIntError
    }
}

impl TryFrom<&str> for EdgeId {
    type Error = ParseIdError;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        if let Some(("", rest)) = v.split_once('e') {
            Ok(Self(GraphItemId::try_from(rest)?))
        } else {
            Err(ParseIdError::MissingPrefix)
        }
    }
}

impl EdgeId {
    pub fn copy_for_frame_id(&self, frame_id: &FrameId) -> Self {
        Self(self.0.copy_for_frame_id(frame_id))
    }
}

impl HasFrameId for EdgeId {
    fn get_frame_id(&self) -> Option<FrameId> {
        self.0.frame_id
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(frame_id) = self.get_frame_id() {
            write!(f, "e{}:{}", self.0.id, frame_id)
        } else {
            write!(f, "e{}", self.0.id)
        }
    }
}

/// An edge, representing an action taken during page load.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Edge {
    pub id: EdgeId,
    pub edge_timestamp: Option<isize>,
    pub edge_type: EdgeType,
    pub source: NodeId,
    pub target: NodeId,
}

impl PartialEq for Edge {
    fn eq(&self, rhs: &Self) -> bool {
        self.id == rhs.id
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, serde::Serialize)]
pub struct FrameId(u128);

impl TryFrom<&str> for FrameId {
    type Error = ParseIdError;
    /// Chromium formats these 128-bit tokens as 32-character hexadecimal strings.
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        if v.len() != 32 {
            return Err(ParseIdError::FrameIdLength);
        }
        Ok(Self(u128::from_str_radix(v, 16)?))
    }
}

impl std::fmt::Display for FrameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0>32X}", self.0)
    }
}

impl std::fmt::Debug for FrameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{:0>32X}\"", self.0)
    }
}

#[cfg(test)]
mod id_parsing_tests {
    use super::*;

    #[test]
    fn test_frame_id_parsing() {
        assert_eq!(FrameId::try_from("00000000000000000000000000000000"), Ok(FrameId(0)));
        assert_eq!(FrameId::try_from("00000000000000000000000000000001"), Ok(FrameId(1)));
        assert_eq!(FrameId::try_from("0000000000000000000000000000000f"), Ok(FrameId(15)));
        assert_eq!(FrameId::try_from("FfFFFFFfFffFFFfFFFFfffFFFfFFFfff"), Ok(FrameId(u128::MAX)));

        assert_eq!(FrameId::try_from(" 00000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
        assert_eq!(FrameId::try_from(" 0000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(FrameId::try_from("0000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
        assert_eq!(FrameId::try_from("000000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
    }

    #[test]
    fn test_graph_item_id_parsing() {
        assert_eq!(GraphItemId::try_from("0"), Ok(GraphItemId{ id: 0, frame_id: None }));
        assert_eq!(GraphItemId::try_from("8"), Ok(GraphItemId{ id: 8, frame_id: None }));
        assert_eq!(GraphItemId::try_from("200"), Ok(GraphItemId{ id: 200, frame_id: None }));
        assert_eq!(GraphItemId::try_from("103810150"), Ok(GraphItemId{ id: 103810150, frame_id: None }));
        assert_eq!(GraphItemId::try_from("0:00000000000000000000000000000000"), Ok(GraphItemId{ id: 0, frame_id: Some(FrameId(0)) }));
        assert_eq!(GraphItemId::try_from("8:00000000000000000000000000000001"), Ok(GraphItemId{ id: 8, frame_id: Some(FrameId(1)) }));
        assert_eq!(GraphItemId::try_from("200:0000000000000000000000000000000f"), Ok(GraphItemId{ id: 200, frame_id: Some(FrameId(15)) }));
        assert_eq!(GraphItemId::try_from("103810150:FfFFFFFfFffFFFfFFFFfffFFFfFFFfff"), Ok(GraphItemId{ id: 103810150, frame_id: Some(FrameId(u128::MAX)) }));

        assert_eq!(GraphItemId::try_from("38 : 00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(GraphItemId::try_from("f8:00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(GraphItemId::try_from(":00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(GraphItemId::try_from("0:0000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
        assert_eq!(GraphItemId::try_from("0:000000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
    }

    #[test]
    fn test_edge_id_parsing() {
        assert_eq!(EdgeId::try_from("e0"), Ok(EdgeId(GraphItemId{ id: 0, frame_id: None })));
        assert_eq!(EdgeId::try_from("e8"), Ok(EdgeId(GraphItemId{ id: 8, frame_id: None })));
        assert_eq!(EdgeId::try_from("e200"), Ok(EdgeId(GraphItemId{ id: 200, frame_id: None })));
        assert_eq!(EdgeId::try_from("e103810150"), Ok(EdgeId(GraphItemId{ id: 103810150, frame_id: None })));
        assert_eq!(EdgeId::try_from("e0:00000000000000000000000000000000"), Ok(EdgeId(GraphItemId{ id: 0, frame_id: Some(FrameId(0)) })));
        assert_eq!(EdgeId::try_from("e8:00000000000000000000000000000001"), Ok(EdgeId(GraphItemId{ id: 8, frame_id: Some(FrameId(1)) })));
        assert_eq!(EdgeId::try_from("e200:0000000000000000000000000000000f"), Ok(EdgeId(GraphItemId{ id: 200, frame_id: Some(FrameId(15)) })));
        assert_eq!(EdgeId::try_from("e103810150:FfFFFFFfFffFFFfFFFFfffFFFfFFFfff"), Ok(EdgeId(GraphItemId{ id: 103810150, frame_id: Some(FrameId(u128::MAX)) })));

        assert_eq!(EdgeId::try_from("n0"), Err(ParseIdError::MissingPrefix));
        assert_eq!(EdgeId::try_from("8"), Err(ParseIdError::MissingPrefix));
        assert_eq!(EdgeId::try_from("e 200"), Err(ParseIdError::ParseIntError));
        assert_eq!(EdgeId::try_from("e103810150:"), Err(ParseIdError::FrameIdLength));
        assert_eq!(EdgeId::try_from("n0:00000000000000000000000000000000"), Err(ParseIdError::MissingPrefix));
        assert_eq!(EdgeId::try_from("0:00000000000000000000000000000000"), Err(ParseIdError::MissingPrefix));
        assert_eq!(EdgeId::try_from("0e:00000000000000000000000000000000"), Err(ParseIdError::MissingPrefix));
        assert_eq!(EdgeId::try_from("e:00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(EdgeId::try_from(":00000000000000000000000000000000"), Err(ParseIdError::MissingPrefix));
        assert_eq!(EdgeId::try_from("e38 : 00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(EdgeId::try_from("ef8:00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(EdgeId::try_from("e:00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(EdgeId::try_from("e0:0000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
        assert_eq!(EdgeId::try_from("e0:000000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
    }

    #[test]
    fn test_node_id_parsing() {
        assert_eq!(NodeId::try_from("n0"), Ok(NodeId(GraphItemId{ id: 0, frame_id: None })));
        assert_eq!(NodeId::try_from("n8"), Ok(NodeId(GraphItemId{ id: 8, frame_id: None })));
        assert_eq!(NodeId::try_from("n200"), Ok(NodeId(GraphItemId{ id: 200, frame_id: None })));
        assert_eq!(NodeId::try_from("n103810150"), Ok(NodeId(GraphItemId{ id: 103810150, frame_id: None })));
        assert_eq!(NodeId::try_from("n0:00000000000000000000000000000000"), Ok(NodeId(GraphItemId{ id: 0, frame_id: Some(FrameId(0)) })));
        assert_eq!(NodeId::try_from("n8:00000000000000000000000000000001"), Ok(NodeId(GraphItemId{ id: 8, frame_id: Some(FrameId(1)) })));
        assert_eq!(NodeId::try_from("n200:0000000000000000000000000000000f"), Ok(NodeId(GraphItemId{ id: 200, frame_id: Some(FrameId(15)) })));
        assert_eq!(NodeId::try_from("n103810150:FfFFFFFfFffFFFfFFFFfffFFFfFFFfff"), Ok(NodeId(GraphItemId{ id: 103810150, frame_id: Some(FrameId(u128::MAX)) })));

        assert_eq!(NodeId::try_from("e0"), Err(ParseIdError::MissingPrefix));
        assert_eq!(NodeId::try_from("8"), Err(ParseIdError::MissingPrefix));
        assert_eq!(NodeId::try_from("n 200"), Err(ParseIdError::ParseIntError));
        assert_eq!(NodeId::try_from("n103810150:"), Err(ParseIdError::FrameIdLength));
        assert_eq!(NodeId::try_from("e0:00000000000000000000000000000000"), Err(ParseIdError::MissingPrefix));
        assert_eq!(NodeId::try_from("0:00000000000000000000000000000000"), Err(ParseIdError::MissingPrefix));
        assert_eq!(NodeId::try_from("0n:00000000000000000000000000000000"), Err(ParseIdError::MissingPrefix));
        assert_eq!(NodeId::try_from("n:00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(NodeId::try_from(":00000000000000000000000000000000"), Err(ParseIdError::MissingPrefix));
        assert_eq!(NodeId::try_from("n38 : 00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(NodeId::try_from("nf8:00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(NodeId::try_from("n:00000000000000000000000000000000"), Err(ParseIdError::ParseIntError));
        assert_eq!(NodeId::try_from("n0:0000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
        assert_eq!(NodeId::try_from("n0:000000000000000000000000000000000"), Err(ParseIdError::FrameIdLength));
    }

    #[test]
    fn test_round_trip() {
        fn test_str(id_str: &str) {
            assert_eq!(format!("{}", NodeId::try_from(id_str).unwrap()), id_str);

            let node_id = NodeId::try_from(id_str).unwrap();

            assert_eq!(NodeId::try_from(format!("{}", node_id).as_str()).unwrap(), node_id);
        }

        test_str("n0");
        test_str("n8");
        test_str("n200");
        test_str("n103810150");
        test_str("n0:00000000000000000000000000000000");
        test_str("n8:00000000000000000000000000000001");
        test_str("n200:0000000000000000000000000000000F");
        test_str("n103810150:FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        test_str("n99999:0123456789ABCDEF0123456789ABCDEF");
    }
}
