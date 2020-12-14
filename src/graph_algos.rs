use crate::graph::{ PageGraph, Edge, EdgeId, Node, NodeId, FrameId };
use crate::types::{ EdgeType, NodeType };

use petgraph::Direction;

impl PageGraph {
    pub fn all_remote_frame_ids(&self) -> Vec<FrameId> {
        self.nodes.iter().filter_map(|(_node_id, node)|
            if let NodeType::RemoteFrame { frame_id } = node.node_type {
                Some(frame_id)
            } else {
                None
            }
        ).collect()
    }

    /// Inserts the graph for a given frame into this graph, namespacing ids to avoid conflicts.
    /// The matching `remote frame` node will gain two new outgoing `cross DOM` edges to the `DOM
    /// root` and `parser` nodes from the frame.
    pub fn merge_frame(&mut self, frame_graph: PageGraph, frame_id: &FrameId) {
        assert!(self.desc.is_root);
        assert!(!frame_graph.desc.is_root);

        // Find the single `remote frame` node with the specified `frame_id`
        let matching_remote_frames = self.filter_nodes(|n| matches!(n, NodeType::RemoteFrame { frame_id: node_frame_id } if node_frame_id == frame_id));
        assert!(matching_remote_frames.len() == 1);
        let remote_frame = matching_remote_frames[0].id.clone();

        // Find the frame's single "DOM root" node with no incoming "cross DOM" edges
        let matching_dom_roots: Vec<_> = frame_graph.nodes.values().filter(|node| {
            if let NodeType::DomRoot { .. } = node.node_type {
                frame_graph.incoming_edges(node).filter(|edge| {
                    matches!(edge.edge_type, EdgeType::CrossDom {})
                }).next().is_none()
            } else {
                false
            }
        }).collect();
        assert_eq!(matching_dom_roots.len(), 1, "Wrong number of top-level DOM roots");
        let dom_root = matching_dom_roots[0].id.clone();

        // Find the frame's single "parser" node with no incoming "cross DOM" edges
        let matching_parsers: Vec<_> = frame_graph.nodes.values().filter(|node| {
            if let NodeType::Parser { .. } = node.node_type {
                frame_graph.incoming_edges(node).filter(|edge| {
                    matches!(edge.edge_type, EdgeType::CrossDom {})
                }).next().is_none()
            } else {
                false
            }
        }).collect();
        assert_eq!(matching_parsers.len(), 1, "Wrong number of top-level parsers");
        let parser = matching_parsers[0].id.clone();

        // TODO "Brave Shields" node should be merged as well

        // For each node in the frame graph
        frame_graph.graph.nodes().for_each(|node_id| {
            // create a new id for the node by prepending the frame id
            let new_node_id = node_id.copy_for_frame_id(frame_id);
            let mut new_node = frame_graph.nodes.get(&node_id).unwrap().clone();
            new_node.id = new_node_id;

            // insert a copy of the node, with the new id, into the root graph
            self.graph.add_node(new_node.id);
            self.nodes.insert(new_node.id, new_node);

            // if the original node has the previously discovered "DOM root" or "parser" id:
            if node_id == dom_root || node_id == parser {
                // insert a new edge from "remote frame" to the new node
                let new_edge = Edge {
                    id: self.new_edge_id(),
                    edge_timestamp: None,
                    edge_type: EdgeType::CrossDom {},
                    source: remote_frame,
                    target: new_node_id,
                };
                match self.graph.edge_weight_mut(remote_frame, new_node_id) {
                    Some(edges) => edges.push(new_edge.id),
                    None => { self.graph.add_edge(remote_frame, new_node_id, vec![new_edge.id]); },
                }
                self.edges.insert(new_edge.id, new_edge);
            }
        });

        // For each edge in the frame graph
        frame_graph.graph.all_edges().for_each(|(from_node_id, to_node_id, edge_ids)| {
            // create a new id for the source, target, and edge by prepending the frame id
            let new_from_node_id = from_node_id.copy_for_frame_id(frame_id);
            let new_to_node_id = to_node_id.copy_for_frame_id(frame_id);

            // insert a copy of the edge, with the new ids, into the root graph
            let new_edge_ids = edge_ids.iter().map(|edge_id| {
                let mut new_edge = frame_graph.edges.get(edge_id).unwrap().clone();
                let new_edge_id = edge_id.copy_for_frame_id(frame_id);
                new_edge.id = new_edge_id;
                new_edge.source = new_from_node_id;
                new_edge.target = new_to_node_id;
                self.edges.insert(new_edge.id, new_edge);
                new_edge_id
            }).collect::<Vec<_>>();
            self.graph.add_edge(new_from_node_id, new_to_node_id, new_edge_ids);
        });
    }

    pub fn filter_edges<F: Fn(&EdgeType) -> bool>(&self, f: F) -> Vec<&Edge> {
        self.edges.values().filter(|edge| {
            f(&edge.edge_type)
        }).collect()
    }

    pub fn filter_nodes<F: Fn(&NodeType) -> bool>(&self, f: F) -> Vec<&Node> {
        self.nodes.values().filter(|node| {
            f(&node.node_type)
        }).collect()
    }

    /// Returns a sorted Vec including 1 edge representing every time the given HtmlElement node was
    /// modified in the page.
    pub fn all_html_element_modifications(&self, node_id: NodeId) -> Vec<(&EdgeId, &Edge)> {
        let element = self.nodes.get(&node_id).unwrap();

        if let NodeType::HtmlElement { node_id: _html_node_id, .. } = element.node_type {
            let mut modifications: Vec<_> = self.graph.neighbors_directed(node_id, Direction::Incoming).map(|node| {
                    let edge_ids = self.graph.edge_weight(node, node_id).unwrap();
                    edge_ids
                })
                .flatten()
                .map(|edge_id| (edge_id, self.edges.get(edge_id).unwrap()))
                .filter(|(_id, edge)| {
                    match edge.edge_type {
                        EdgeType::Structure { .. } => false,
                        _ => true,
                    }
                })
                .collect();

            modifications.sort_by_key(|(_, edge)| edge.edge_timestamp.expect("HTML element modification had no timestamp"));

            modifications
        } else {
            panic!("Supply a node with HtmlElement node type");
        }
    }

    /// Get a collection of any Script nodes responsible for fetching the given Resource node.
    pub fn scripts_that_caused_resource(&self, node_id: NodeId) -> Vec<(NodeId, &Node)> {
        let element = self.nodes.get(&node_id).unwrap();

        if let NodeType::Resource { url: ref _url } = element.node_type {
            let incoming: Vec<_> = self.graph.neighbors_directed(node_id, Direction::Incoming).map(|node| {
                (node, self.nodes.get(&node).unwrap())
            }).filter(|(_id, _node)| {
                true
            }).collect();

            incoming
        } else {
            panic!("Supply a node with Resource node type");
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

    /// Gets the URL of the page the graph was recorded from
    pub fn root_url(&self) -> String {
        return self.desc.url.to_string();
    }

    /// Get every request type and associated resource size for a given resource.
    ///
    /// Some requests, like streamed fetches, video, or audio cannot be properly sized, so their
    /// sizes will be None.
    pub fn resource_request_types(&self, resource_node: &NodeId) -> Vec<(String, Option<usize>)> {
        if let NodeType::Resource { .. } = self.nodes.get(resource_node).unwrap().node_type {
            let request_start_edges = self.graph
                .edges_directed(resource_node.to_owned(), Direction::Incoming)
                .map(|(_, _, edge_ids)| edge_ids)
                .flatten()
                .filter(|edge_id| match &self.edges.get(edge_id).unwrap().edge_type {
                    EdgeType::RequestStart { .. } => true,
                    _ => false,
                });
            let unique_request_types = request_start_edges.map(|edge_id|
                    if let Some(Edge { edge_type: EdgeType::RequestStart { request_type, request_id, .. }, .. }) = self.edges.get(edge_id) {
                        let request_type = request_type.as_str().to_owned();

                        let mut matching_request_sizes = self.edges
                            .iter()
                            .filter_map(|(_, Edge { edge_type, .. })| if let EdgeType::RequestComplete { size, request_id: id, .. } = edge_type {
                                    if id == request_id {
                                        Some(size.parse::<usize>().ok())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                });

                        let size = matching_request_sizes.next().unwrap_or_default();

                        (request_type, size)
                    } else {
                        unreachable!()
                    }
                ).collect::<std::collections::HashSet<_>>()
                .into_iter().collect::<Vec<_>>();
            if unique_request_types.len() == 0 {
                return vec![("other".to_string(), None)]
            }

            unique_request_types
        } else {
            panic!("resource_request_type must be supplied a node of type Resource");
        }
    }

    /// Get a collection of all Resource nodes whose requests match a given adblock filter pattern.
    pub fn resources_matching_filter(&self, pattern: &str) -> Vec<(NodeId, &Node)> {
        let source_url = self.root_url();

        let source_url = url::Url::parse(&source_url).expect("Could not parse source URL");
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
                        let request_url_hostname = match request_url.host_str() {
                            Some(host) => host,
                            None => return false,
                        };
                        let request_url_domain = get_domain(request_url_hostname);

                        let request_types = self.resource_request_types(id);

                        request_types.iter().map(|(request_type, _size)| rule.matches(&adblock::request::Request::new(
                            request_type,
                            url,
                            request_url_scheme,
                            request_url_hostname,
                            &request_url_domain,
                            source_hostname,
                            &source_domain,
                        ))).any(|matched| matched)
                    }
                    _ => false
                })
                .map(|(id, node)| (id.clone(), node))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn direct_downstream_effects_of(&self, node_id: &NodeId) -> Vec<(NodeId, &Node)>{
        match &self.nodes.get(node_id).unwrap().node_type {
            NodeType::Extensions {} => unimplemented!(),
            NodeType::RemoteFrame { .. } => unimplemented!(),
            NodeType::Resource { .. } => {
                // script resources cause the execution of the corresponding script, which is connected
                // through the corresponding HTML script element.
                let attached_script_elements = self.graph.edges_directed(*node_id, Direction::Outgoing)
                    .map(|(_n0, n1, edge_ids)| edge_ids.iter().map(move |edge_id| match &self.edges.get(&edge_id).unwrap().edge_type {
                        EdgeType::RequestComplete { .. } => Some(n1),
                        _ => None,
                    }))
                    .flatten()
                    .filter_map(|v| v)
                    .filter(|target_node| match &self.nodes.get(target_node).unwrap().node_type {
                        NodeType::HtmlElement { tag_name, .. } if tag_name == "script" => true,
                        _ => false,
                    }).collect::<Vec<_>>();

                attached_script_elements.into_iter()
                    .map(|html_node_id| self.graph
                        .neighbors_directed(html_node_id, Direction::Outgoing)
                        .filter(|html_neighbor| match &self.nodes.get(html_neighbor).unwrap().node_type {
                            NodeType::Script { .. } => true,
                            _ => false,
                        })
                    ).flatten()
                    .map(|script_node_id| (script_node_id, self.nodes.get(&script_node_id).unwrap()))
                    .collect::<Vec<_>>()
            }
            NodeType::AdFilter { .. } => unimplemented!(),
            NodeType::TrackerFilter => unimplemented!(),  // TODO
            NodeType::FingerprintingFilter => unimplemented!(),   // TODO
            NodeType::WebApi { .. } => unimplemented!(),
            NodeType::JsBuiltin { .. } => unimplemented!(),
            NodeType::HtmlElement { tag_name, .. } if tag_name == "script" => {
                // script elements with a src attribute cause a resource request
                let resource_requests = self.graph.neighbors_directed(*node_id, Direction::Outgoing).filter(|node_id| match &self.nodes.get(&node_id).unwrap().node_type {
                    NodeType::Resource { .. } => true,
                    _ => false,
                }).map(|node_id| (node_id, self.nodes.get(&node_id).unwrap()))
                .collect::<Vec<_>>();

                // inline script elements cause a script execution
                if resource_requests.is_empty() {
                    self.graph.neighbors_directed(*node_id, Direction::Outgoing).filter(|node_id| match &self.nodes.get(&node_id).unwrap().node_type {
                        NodeType::Script { .. } => true,
                        _ => false,
                    }).map(|node_id| (node_id, self.nodes.get(&node_id).unwrap()))
                    .collect::<Vec<_>>()
                } else {
                    resource_requests
                }
            }
            NodeType::HtmlElement { tag_name: _, .. } => unimplemented!(),
            NodeType::TextNode { .. } => unimplemented!(),
            NodeType::DomRoot { .. } => unimplemented!(),
            NodeType::FrameOwner { .. } => unimplemented!(),
            NodeType::Storage {} => unimplemented!(),
            NodeType::LocalStorage {} => unimplemented!(),
            NodeType::SessionStorage {} => unimplemented!(),
            NodeType::CookieJar {} => unimplemented!(),
            NodeType::Script { .. } => {
                // scripts can fetch resources
                let fetched_resources = self.graph.edges_directed(*node_id, Direction::Incoming).map(|(n0, _n1, edge_ids)| edge_ids.iter().map(move |edge_id| match &self.edges.get(&edge_id).unwrap().edge_type {
                        EdgeType::RequestComplete { .. } => Some(n0),
                        _ => None,
                    }))
                    .flatten()
                    .filter_map(|v| v)
                    .map(|node_id| (node_id, self.nodes.get(&node_id).unwrap()));

                // scripts can execute other scripts
                let executed_scripts = self.graph.neighbors_directed(*node_id, Direction::Outgoing).filter(|node_id| match &self.nodes.get(&node_id).unwrap().node_type {
                    NodeType::Script { .. } => true,
                    _ => false,
                }).map(|node_id| (node_id, self.nodes.get(&node_id).unwrap()));

                fetched_resources.chain(executed_scripts).collect::<Vec<_>>()
                // TODO scripts can create/modify/insert DOM elements, execute web APIs and JS
                // builtins, build 3rd party frames, access storage, access cookies...
            }
            NodeType::Parser {} => unimplemented!(),
            NodeType::BraveShields {} => unimplemented!(),
            NodeType::AdsShield {} => unimplemented!(),
            NodeType::TrackersShield {} => unimplemented!(),
            NodeType::JavascriptShield {} => unimplemented!(),
            NodeType::FingerprintingShield {} => unimplemented!(),
            NodeType::FingerprintingV2Shield {} => unimplemented!(),
        }
    }

    pub fn all_downstream_effects_of(&self, node_id: &NodeId) -> Vec<(NodeId, &Node)>{
        let mut nodes_to_check = vec![*node_id];
        let mut already_checked = vec![];

        while let Some(node_id) = nodes_to_check.pop() {
            let direct_effects = self.direct_downstream_effects_of(&node_id);
            already_checked.push(node_id);

            direct_effects.into_iter().for_each(|(node_id, _)| if !already_checked.contains(&node_id) {
                nodes_to_check.push(node_id);
            });
        }

        already_checked.into_iter().map(|node_id| (node_id, self.nodes.get(&node_id).unwrap())).collect()
    }
}

fn get_domain(host: &str) -> String {
    let source_hostname = host;
    let source_domain = source_hostname.parse::<addr::DomainName>().expect("Source URL domain could not be parsed");
    let source_domain = &source_hostname[source_hostname.len() - source_domain.root().to_str().len()..];
    source_domain.to_string()
}
