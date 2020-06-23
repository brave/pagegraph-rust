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

    /// Get a collection of all Resource nodes whose requests match a given adblock filter pattern.
    pub fn resources_matching_filter(&self, pattern: &str) -> Vec<(NodeId, &Node)> {
        let root_urls = self.nodes
            .iter()
            .filter(|(node_id, node)| {
                if let NodeType::DomRoot { .. } = node.node_type {
                    self.graph.edges_directed(*node_id.to_owned(), Direction::Incoming).count() == 0
                } else {
                    false
                }
            })
            .map(|(_, node)| {
                match &node.node_type {
                    NodeType::DomRoot { url: Some(url), .. } => url,
                    _ => panic!("Could not find DOM root URL"),
                }
            }).collect::<Vec<_>>();
        assert_eq!(root_urls.len(), 1);
        let source_url = root_urls[0];

        let source_url = url::Url::parse(source_url).expect("Could not parse source URL");
        let source_hostname = source_url.host_str().expect(&format!("Source URL has no host, {:?}", source_url));
        let source_domain = get_domain(source_hostname);

        if let Ok(rule) = adblock::filters::network::NetworkFilter::parse(pattern, false) {
            self.nodes
                .iter()
                .filter(|(id, node)| match &node.node_type {
                    NodeType::Resource { url } => {
                        use adblock::filters::network::NetworkMatchable as _;

                        let request_url = match url::Url::parse(url) {
                            Ok(request_url) => request_url,
                            Err(_) => return false,
                        };
                        let request_url_scheme = request_url.scheme();
                        let request_url_hostname = request_url.host_str().expect("Request URL has no host");
                        let request_url_domain = get_domain(request_url_hostname);

                        let request_start_edges = self.graph
                            .edges_directed(*id.to_owned(), Direction::Incoming)
                            .filter(|(_, _, edge_id)| match &self.edges.get(edge_id).unwrap().edge_type {
                                EdgeType::RequestStart { .. } => true,
                                _ => false,
                            });
                        let mut unique_request_types = request_start_edges.map(|(_, _, edge_id)|
                                if let Some(Edge { edge_type: EdgeType::RequestStart { request_type, .. }, .. }) = self.edges.get(edge_id) {
                                    request_type.to_owned()
                                } else {
                                    unreachable!()
                                }
                            ).collect::<std::collections::HashSet<_>>()
                            .into_iter().collect::<Vec<_>>();
                        if unique_request_types.len() == 0 {
                            panic!("Resource did not have a corresponding RequestStart edge");
                        } else if unique_request_types.len() > 1 {
                            panic!("Resource was requested with multiple different request types: {:?}", unique_request_types);
                        }
                        let request_type = unique_request_types.remove(0);

                        let request = adblock::request::Request::new(
                            &request_type,
                            url,
                            request_url_scheme,
                            request_url_hostname,
                            &request_url_domain,
                            source_hostname,
                            &source_domain,
                        );
                        rule.matches(&request)
                    }
                    _ => false
                })
                .map(|(id, node)| (id.clone(), node))
                .collect()
        } else {
            vec![]
        }
    }
}

fn get_domain(host: &str) -> String {
    let source_hostname = host;
    let source_domain = source_hostname.parse::<addr::DomainName>().expect("Source URL domain could not be parsed");
    let source_domain = &source_hostname[source_hostname.len() - source_domain.root().to_str().len()..];
    source_domain.to_string()
}
