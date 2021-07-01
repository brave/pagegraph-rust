use crate::graph::{PageGraph, Edge, EdgeId, Node, NodeId, FrameId, DownstreamRequests};
use crate::types::{ EdgeType, NodeType };

use petgraph::Direction;
use adblock::engine::Engine;

const CAN_HAVE_SRC: [&str; 9] = ["audio", "embed", "iframe", "img", "input", "script", "source", "track", "video"];

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

    pub fn dom_root_for_html_node<'a>(&'a self, node: &'a Node) -> Option<&'a Node> {
        match node.node_type {
            NodeType::DomRoot { .. } => return Some(node),
            NodeType::HtmlElement { .. } | NodeType::TextNode { .. } | NodeType::FrameOwner { .. } => {
                let mut parent_ids = self.incoming_edges(node).filter_map(|edge| if let EdgeType::InsertNode { parent, .. } = edge.edge_type { Some(parent) } else { None });
                // Look for all parent elements, as per parent id from InsertNode
                while let Some(parent_id) = parent_ids.next() {
                    let parent_node = {
                        let mut parent_nodes = self.nodes.values().filter(|parent_node|
                            crate::graph::is_same_frame_context(node.id, parent_node.id) &&
                            matches!(parent_node.node_type, NodeType::HtmlElement { node_id, .. } | NodeType::DomRoot { node_id, .. } | NodeType::FrameOwner { node_id, .. } if node_id == parent_id)
                        );
                        let parent_node = parent_nodes.next().expect(&format!("No HTML parent node with id {} found for {:?}", parent_id, node));
                        assert!(parent_nodes.next().is_none(), "Multiple HTML parent nodes with id {} found", parent_id);
                        parent_node
                    };

                    if let Some(dom_root) = self.dom_root_for_html_node(parent_node) {
                        return Some(dom_root);
                    }
                }

                // If the element was never inserted, it may have been created by a script.
                let creator = {
                    let mut creators = self.incoming_edges(node).filter_map(|edge| if let EdgeType::CreateNode { .. } = edge.edge_type { Some(self.source_node(edge)) } else { None });
                    let creator = creators.next().unwrap();
                    assert!(creators.next().is_none(), "HTML element was created multiple times");
                    creator
                };

                match creator.node_type {
                    NodeType::Script { .. } => {
                        let dom_roots = self.incoming_edges(creator)
                            .filter(|edge| matches!(edge.edge_type, EdgeType::Execute {}))
                            .map(|edge| self.dom_root_for_edge(edge).unwrap())
                            .collect::<Vec<_>>();

                        if dom_roots.len() == 0 {
                            // There's some complicated cases with module scripts that can sometimes
                            // lead to infinite loops, so just find the local context URL and use that.
                            // TODO improve
                            return Some(self.local_context_root_for_id(creator.id));
                        }
                        assert!(dom_roots.len() >= 1, "Script had no executor (a) {:?}", creator);
                        // Sometimes the same script src is executed from multiple DOM roots in the
                        // same local frame context
                        // In practice, we only use the URL here to check the partiness of a
                        // request, which will be the same for all roots. So we just take the first
                        // one alphabetically for the sake of determinism.
                        // TODO improve
                        let mut all_dom_roots = dom_roots.iter().filter(|root| matches!(&root.node_type, NodeType::DomRoot { url, .. } if url.is_some())).collect::<Vec<_>>();
                        all_dom_roots.sort_unstable_by_key(|root| match &root.node_type { NodeType::DomRoot { url, .. } => url.to_owned(), _ => unreachable!() });

                        Some(all_dom_roots[0])
                    }
                    _ => panic!("HTML element never inserted, but created by {:?}, which is not a script", creator),
                }
            }
            _ => panic!("Supplied node was not an HTML element"),
        }
    }

    /// Returns the DOM root node(s) according to the frame that the given edge originated from.
    pub fn dom_root_for_edge(&self, edge: &Edge) -> Option<&Node> {
        match &edge.edge_type {
            EdgeType::RequestComplete { .. } => {
                let target = self.target_node(edge);
                match &target.node_type {
                    NodeType::HtmlElement { .. } | NodeType::FrameOwner { .. } => Some(self.dom_root_for_html_node(target).unwrap_or_else(|| panic!("could not find DOM root for request initiator element {:?}", target))),
                    NodeType::Script { .. } => {
                        // Scripts generally are pointed to by a single Execute edge, but there can
                        // be more than one for multiple script elements with the same source.
                        let dom_roots = self.incoming_edges(self.target_node(edge))
                            .filter(|edge| matches!(edge.edge_type, EdgeType::Execute {}))
                            .map(|edge| self.dom_root_for_edge(edge).unwrap())
                            .collect::<Vec<_>>();

                        if dom_roots.len() == 0 {
                            // There's some complicated cases with module scripts that can sometimes
                            // lead to infinite loops, so just find the local context URL and use that.
                            // TODO improve
                            return Some(self.local_context_root_for_id(edge.id));
                        }
                        assert!(dom_roots.len() >= 1, "Script had no executor (b) {:?}", self.target_node(edge));
                        // Sometimes the same script src is executed from multiple DOM roots in the
                        // same local frame context
                        // In practice, we only use the URL here to check the partiness of a
                        // request, which will be the same for all roots. So we just take the first
                        // one alphabetically for the sake of determinism.
                        // TODO improve
                        let mut all_dom_roots = dom_roots.iter().filter(|root| matches!(&root.node_type, NodeType::DomRoot { url, .. } if url.is_some())).collect::<Vec<_>>();
                        all_dom_roots.sort_unstable_by_key(|root| match &root.node_type { NodeType::DomRoot { url, .. } => url.to_owned(), _ => unreachable!() });

                        Some(all_dom_roots[0])
                    }
                    // Prefetches and requests from CSS are initiated by the parser. We have no way
                    // to attribute those to particular DOM roots, so we ignore them for now.
                    NodeType::Parser { .. } => None,
                    _ => panic!("Request initiated by {:?} (something other than a script or HTML element)", &target),
                }
            }
            EdgeType::Execute { .. } => {
                let source = self.source_node(edge);
                match &source.node_type {
                    NodeType::HtmlElement { tag_name, .. } if tag_name == "script" => Some(self.dom_root_for_html_node(source).expect("could not find DOM root for script executor element")),
                    NodeType::Script { script_type, .. } if script_type == "module" => {
                        // There's some complicated cases with module scripts that can sometimes
                        // lead to infinite loops, so just find the local context URL and use that.
                        // TODO improve
                        Some(self.local_context_root_for_id(edge.id))
                    }
                    NodeType::Script { .. } => {
                        // Scripts generally are pointed to by a single Execute edge, but there can
                        // be more than one for multiple script elements with the same source.
                        let dom_roots = self.incoming_edges(source)
                            .filter(|edge| matches!(edge.edge_type, EdgeType::Execute {}))
                            .map(|edge| self.dom_root_for_edge(edge).unwrap())
                            .collect::<Vec<_>>();

                        assert!(dom_roots.len() >= 1, "Script had no executor (b) {:?}", self.target_node(edge));
                        // Sometimes the same script src is executed from multiple DOM roots in the
                        // same local frame context
                        // In practice, we only use the URL here to check the partiness of a
                        // request, which will be the same for all roots. So we just take the first
                        // one alphabetically for the sake of determinism.
                        // TODO improve
                        let mut all_dom_roots = dom_roots.iter().filter(|root| matches!(&root.node_type, NodeType::DomRoot { url, .. } if url.is_some())).collect::<Vec<_>>();
                        all_dom_roots.sort_unstable_by_key(|root| match &root.node_type { NodeType::DomRoot { url, .. } => url.to_owned(), _ => unreachable!() });

                        Some(all_dom_roots[0])
                    }
                    // Unclear why, but DOM roots sometimes execute scripts as well.
                    NodeType::DomRoot { .. } => Some(source),
                    _ => panic!("Script was executed by {:?} (something other than a script HTML element or another script)", &source.node_type),
                }
            }
            EdgeType::CrossDom {} => {
                let source = self.source_node(edge);
                match &source.node_type {
                    NodeType::RemoteFrame { .. } => {
                        let previous_edge = {
                            let mut previous_edges = self.incoming_edges(source).filter(|edge| matches!(edge.edge_type, EdgeType::CrossDom {}));
                            let previous_edge = previous_edges.next().expect("remote frame had no incoming cross DOM edge");
                            assert!(previous_edges.next().is_none());
                            previous_edge
                        };

                        self.dom_root_for_edge(previous_edge)
                    }
                    NodeType::FrameOwner { .. } => {
                        Some(self.dom_root_for_html_node(source).expect("could not find DOM root for frame owner element"))
                    }
                    // When a script creates a DOM root, it can be attached directly to another
                    // root.
                    NodeType::DomRoot { .. } => {
                        Some(source)
                    }
                    _ => panic!("Cross DOM edge {:?} has a {:?} source (not a remote frame, frame owner, or another DOM root)", &edge, &source),
                }
            }
            _ => unimplemented!(),
        }
    }

    /// Returns the top-level DOM root node for a particular local context - not necessarily the
    /// root of a given frame, but at least still first-party to that frame.
    pub fn local_context_root_for_id<I: crate::graph::HasFrameId + Copy>(&self, item: I) -> &Node {
        let matching_dom_roots: Vec<_> = self.nodes.values()
            // Only consider nodes in the same local context
            .filter(|node| crate::graph::is_same_frame_context(item, node.id))
            // Only consider DOM root nodes
            .filter(|node| matches!(node.node_type, NodeType::DomRoot { .. }))
            // Only consider nodes which have no incoming CrossDom edges from the same local
            // context
            .filter(|node| {
                self.incoming_edges(node).filter(|edge| {
                    matches!(edge.edge_type, EdgeType::CrossDom {}) && crate::graph::is_same_frame_context(item, edge.id)
                }).next().is_none()
            })
            .collect();
        assert_eq!(matching_dom_roots.len(), 1, "Wrong number of local context DOM roots");
        matching_dom_roots[0]
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

    /// Get a collection of all Resource nodes whose requests match a set of adblock filter patterns.
    /// Optionally, only match on exception patterns
    pub fn resources_matching_filters(&self, patterns: Vec<String>, only_exceptions: bool) -> Vec<(NodeId, &Node)> {
        let source_url = self.root_url();

        let source_url = url::Url::parse(&source_url).expect("Could not parse source URL");
        let source_hostname = source_url.host_str().expect(&format!("Source URL has no host, {:?}", source_url));
        let source_domain = get_domain(source_hostname);
        let blocker = Engine::from_rules(&patterns);

        self.nodes
            .iter()
            .filter(|(id, node)| match &node.node_type {
                NodeType::Resource { url } => {
                    let request_url = match url::Url::parse(url) {
                        Ok(request_url) => request_url,
                        Err(_) => return false,
                    };
                    let request_url_hostname = match request_url.host_str() {
                        Some(host) => host,
                        None => return false,
                    };
                    let request_url_domain = get_domain(request_url_hostname);

                    let request_types = self.resource_request_types(id);

                    request_types.iter().map(|(request_type, _size)|  {
                        let third_party = if source_domain.is_empty() {
                            None
                        } else {
                            Some(source_domain != request_url_domain)
                        };
                        let blocker_result = blocker
                            .check_network_urls_with_hostnames_subset(url,
                                                                      request_url_hostname,
                                                                      source_hostname,
                                                                      request_type,
                                                                      third_party,
                                                                      false,
                                                                      true);
                        if only_exceptions {
                            blocker_result.exception.is_some()
                        } else {
                            blocker_result.matched
                        }
                    }).any(|matched| matched)
                }
                _ => false
            })
            .map(|(id, node)| (id.clone(), node))
            .collect()
    }

    /// Get a collection of all Resource nodes whose requests match a given adblock filter pattern.
    /// Optionally, only match on exception patterns
    pub fn resources_matching_filter(&self, pattern: &str, only_exceptions: bool) -> Vec<(NodeId, &Node)> {
        return self.resources_matching_filters(vec![pattern.to_string()], only_exceptions);
    }

    pub fn direct_downstream_effects_of(&self, edge: &Edge) -> Vec<&Edge>{
        match &edge.edge_type {
            EdgeType::Filter {} => unimplemented!(),
            EdgeType::Structure {} => panic!("Structure edges should not be examined for downstream effects"),
            EdgeType::CrossDom {} => {
                // Cross DOM edges can point to frame roots, including remote frames
                match self.target_node(edge).node_type {
                    NodeType::DomRoot { .. } => {
                        // Get the entire set of CreateNode, SetAttribute, and InsertNode edges
                        // from a Parser node that make up this initial DOM tree

                        // Find the single Parser node that belongs to the same local frame context
                        // as this DOM root
                        let parsers = self.filter_nodes(|node_type| matches!(node_type, NodeType::Parser {}));
                        let mut same_context_parsers = parsers
                            .iter()
                            .filter(|parser| {
                                crate::graph::is_same_frame_context(edge.target, parser.id)
                            });
                        let same_context_parser = same_context_parsers.next().expect("Frame context had no parsers");
                        assert!(same_context_parsers.next().is_none(), "Frame context had multiple parsers");

                        // Get all nodes targeted from the Parser by outgoing CreateNode edges.
                        // This provides all initial DOM nodes in the same local frame *context*,
                        // but still not necessarily the same local frame.
                        let mut same_context_dom_nodes = self.outgoing_edges(same_context_parser)
                            .filter(|edge| matches!(edge.edge_type, EdgeType::CreateNode {}))
                            .map(|edge| self.target_node(edge))
                            // Some DOM roots have no incoming CreateNode edge, so we need to
                            // consider those as well.
                            .chain(self.nodes.values().filter(|node| {
                                matches!(node.node_type, NodeType::DomRoot { .. }) &&
                                    crate::graph::is_same_frame_context(node.id, edge.target) &&
                                    self.incoming_edges(node)
                                        .filter(|edge| matches!(edge.edge_type, EdgeType::CreateNode {}))
                                        .next()
                                        .is_none()
                            }))
                            // Then map each node to also include quick reference to its HTML node
                            // id, and an unpopulated Option<bool> flag to signify whether or not
                            // its part of this frame.
                            .map(|node| (node, match node.node_type {
                                NodeType::DomRoot { node_id, .. } => node_id,
                                NodeType::HtmlElement { node_id, .. } => node_id,
                                NodeType::TextNode { node_id, .. } => node_id,
                                NodeType::FrameOwner { node_id, .. } => node_id,
                                _ => panic!("Parser created a {:?}, which has no DOM node ID", node.node_type),
                            }, None))
                            .collect::<Vec<(_, _, Option<bool>)>>();    // (Node, Node id, optional flag)

                        // Now we need to filter out any nodes which are in a different local frame
                        // from the DOM root.
                        //
                        // Sort these HTML element/DOM root/Text nodes by ascending HTML node id.
                        same_context_dom_nodes.sort_unstable_by_key(|(_node, node_id, _flag)| *node_id);

                        // There should be no duplicate HTML node ids.
                        same_context_dom_nodes.windows(2)
                            .for_each(|window| assert!(matches!(window, [(_, a_id, _), (_, b_id, _)] if a_id != b_id), "HTML node id {} is present twice", window[0].1));

                        // Now each node's flag can be populated as follows:
                        //  - If the node is already flagged, return the flag
                        //  - If the node is an unflagged DOM root, flag it according to whether
                        //    it's equal to this frame's DOM root and return the flag
                        //  - Check incoming InsertNode edges until one is found with a parent ID
                        //    that exists in the vec, by binary search
                        //  - Recur on the parent element to get its flag
                        //  - Copy the parent flag to this element
                        fn populate_and_get_flag(index: usize, all_nodes: &mut Vec<(&Node, usize, Option<bool>)>, target_dom_root: NodeId, pg: &PageGraph) -> bool {
                            if let Some(flag) = all_nodes[index].2 {
                                return flag;
                            } else {
                                let node = all_nodes[index].0;
                                let new_flag = match node.node_type {
                                    NodeType::DomRoot { .. } => {
                                        node.id == target_dom_root
                                    }
                                    _ => {
                                        let mut parent_nodes = pg.incoming_edges(node)
                                            .filter_map(|edge| match edge.edge_type {
                                                EdgeType::InsertNode { parent, .. } => Some(parent),
                                                _ => None,
                                            });
                                        loop {
                                            match parent_nodes.next() {
                                                Some(parent) => {
                                                    if let Ok(next_index) = all_nodes.binary_search_by_key(&parent, |(_node, id, _flag)| *id) {
                                                        break populate_and_get_flag(next_index, all_nodes, target_dom_root, pg);
                                                    }
                                                }
                                                // If no insert edge exists for a node created by the
                                                // parser, then we have no choice but to ignore it. TODO
                                                // most of these are probably graph issues.
                                                None => break false,
                                            }
                                        }
                                    }
                                };
                                all_nodes[index].2 = Some(new_flag);
                                return new_flag;
                            }
                        }

                        for i in 0..same_context_dom_nodes.len() {
                            populate_and_get_flag(i, &mut same_context_dom_nodes, edge.target, &self);
                        }

                        // Finally, filter that list by flag, filter out the single DOM root, map
                        // each node to a vec of any incoming CreateNode, SetAttribute, or
                        // InsertNode edges originating from the Parser, flatten, and return this
                        // set of edges.
                        same_context_dom_nodes.iter()
                            .filter(|(_, _, flag)| flag.unwrap())
                            .filter(|(node, _, _)| !matches!(node.node_type, NodeType::DomRoot { .. }))
                            .map(|(node, _, _)| self.incoming_edges(node)
                                .filter(|edge| match edge.edge_type {
                                    EdgeType::CreateNode { .. } => true,
                                    EdgeType::SetAttribute { .. } => true,
                                    EdgeType::InsertNode { .. } => true,
                                    _ => false,
                                } && self.source_node(edge).id == same_context_parser.id).collect::<Vec<_>>())
                            .flatten()
                            .collect::<Vec<_>>()
                    }
                    NodeType::Parser {} => {
                        // This happens when a remote frame is merged. The downstream effects of
                        // the frame will be enumerated by the corresponding DOM root, so we can
                        // return an empty list here.
                        vec![]
                    }
                    NodeType::RemoteFrame { .. } => {
                        // Just return the outgoing CrossDom edge to the attached DOM root (if the
                        // remote frame has been merged into this graph). The above algorithm will
                        // work on that root in the next recurrence.
                        self.outgoing_edges(self.target_node(edge))
                            .filter(|edge| matches!(edge.edge_type, EdgeType::CrossDom {}))
                            .collect()
                    }
                    _ => panic!("Cross DOM edges should only point to DOM roots, parsers, and remote frames, {:?}", self.target_node(edge)),
                }
            }
            EdgeType::ResourceBlock {} => unimplemented!(),
            EdgeType::Shield {} => unimplemented!(),
            EdgeType::TextChange {} => unimplemented!(),
            EdgeType::RemoveNode {} => unimplemented!(),
            EdgeType::DeleteNode {} => unimplemented!(),
            EdgeType::InsertNode { parent: parent_id, .. } => {
                // Inserting a node can cause certain elements with `src` attributes to trigger a
                // network request, however we use `SetAttribute` instead as a rough approximation
                // of this.

                // On the other hand, inserting a text node can trigger an inline script execution
                // if its parent HTML node is a script tag. In that case, we attribute the
                // chronologically next outgoing execute edge from the parent script as an effect
                // of this insertion.
                if let NodeType::TextNode { .. } = self.target_node(edge).node_type {
                    let parent_node = {
                        let mut parent_nodes = self.nodes.values().filter(|parent_node|
                            crate::graph::is_same_frame_context(edge.id, parent_node.id) &&
                            matches!(parent_node.node_type, NodeType::HtmlElement { node_id, .. } | NodeType::DomRoot { node_id, .. } | NodeType::FrameOwner { node_id, .. } if node_id == *parent_id)
                        );
                        let parent_node = parent_nodes.next().expect(&format!("No HTML parent node with id {} found for insertion {:?}", parent_id, edge));
                        assert!(parent_nodes.next().is_none(), "Multiple HTML parent nodes with id {} found", parent_id);
                        parent_node
                    };

                    match &parent_node.node_type {
                        NodeType::HtmlElement { tag_name, .. } if tag_name == "script" => {
                            let insertion_time = edge.edge_timestamp;
                            let next_execution = self.outgoing_edges(parent_node)
                                .filter(|edge| matches!(edge.edge_type, EdgeType::Execute {}) && edge.edge_timestamp >= insertion_time)
                                .min_by_key(|edge| edge.edge_timestamp);
                            // Some script elements are not executed, e.g.
                            // `<script type="application/json">`.
                            // This is fine.
                            return match next_execution {
                                Some(next_execution) => vec![next_execution],
                                None => vec![],
                            };
                        },
                        _ => (),
                    }
                }

                vec![]
            }
            EdgeType::CreateNode {} => {
                // Creating a node generally doesn't cause anything to happen.
                vec![]
            }
            EdgeType::JsResult { .. } => unimplemented!(),
            EdgeType::JsCall { .. } => unimplemented!(),
            EdgeType::RequestComplete { resource_type, .. } => {
                // If RequestComplete has a "script" resource type, and points to an HTML script
                // element, then attribute any Executions from that element to this edge.
                let target = self.target_node(edge);
                if resource_type == "script" && matches!(&target.node_type, NodeType::HtmlElement { tag_name, .. } if tag_name == "script") {
                    self.outgoing_edges(target).filter(|edge| matches!(edge.edge_type, EdgeType::Execute {})).collect::<Vec<_>>()
                } else {
                    vec![]
                }
            }
            EdgeType::RequestError { .. } => {
                // Request errors generally don't cause anything to happen.
                vec![]
            },
            EdgeType::RequestStart { request_id, .. } => {
                // Request starts cause request completions or errors.
                self.outgoing_edges(self.target_node(edge)).filter(|edge| match edge.edge_type {
                    EdgeType::RequestComplete { request_id: complete_id, .. } if *request_id == complete_id => true,
                    EdgeType::RequestError { request_id: error_id, .. } if *request_id == error_id => true,
                    _ => false,
                }).collect()
            }
            EdgeType::RequestResponse => unimplemented!(),
            EdgeType::AddEventListener { .. } => unimplemented!(),
            EdgeType::RemoveEventListener { .. } => unimplemented!(),
            EdgeType::EventListener { .. } => unimplemented!(),
            EdgeType::StorageSet { .. } => unimplemented!(),
            EdgeType::StorageReadResult { .. } => unimplemented!(),
            EdgeType::DeleteStorage { .. } => unimplemented!(),
            EdgeType::ReadStorageCall { .. } => unimplemented!(),
            EdgeType::ClearStorage { .. } => unimplemented!(),
            EdgeType::StorageBucket {} => unimplemented!(),
            EdgeType::ExecuteFromAttribute { .. } => unimplemented!(),
            EdgeType::Execute {} => {
                self.outgoing_edges(self.target_node(edge)).filter(|edge| match edge.edge_type {
                    // A script execution can cause a network request
                    EdgeType::RequestStart { .. } => true,
                    // A script execution can cause another script to be executed
                    EdgeType::Execute {} => true,
                    // A script execution can set attributes on other HTML elements, causing them
                    // to initiate a network request
                    EdgeType::SetAttribute { .. } => true,
                    // TODO scripts can create/insert DOM elements, execute web APIs and JS builtins,
                    // build 3rd party frames, access storage, access cookies...
                    _ => false,
                }).collect()
            }
            EdgeType::SetAttribute { key, .. } => {
                let target = self.target_node(edge);
                match &target.node_type {
                    // Setting a src attribute on some HTML elements causes them to initiate a network
                    // request
                    NodeType::HtmlElement { tag_name, .. } if key == "src" && CAN_HAVE_SRC.contains(&tag_name.as_str()) => {
                        if key == "src" && CAN_HAVE_SRC.contains(&tag_name.as_str()) {
                            self.outgoing_edges(target).filter(|edge| match edge.edge_type {
                                EdgeType::RequestStart { .. } => true,
                                _ => false,
                            }).collect()
                        } else {
                            vec![]
                        }
                    }
                    // Setting a src attribute a frame owner causes it to load a new frame context
                    NodeType::FrameOwner { tag_name, .. } if key == "src" && CAN_HAVE_SRC.contains(&tag_name.as_str()) => {
                        // Find outgoing CrossDom edges to DomRoot nodes with `url` != "about:blank", and choose those occurring
                        // *after* the original edge, but before other later `SetAttribute` `src` edges, if any exist.
                        let set_attribute_timestamp = edge.edge_timestamp.unwrap();
                        let mut later_set_attribute_times = self.incoming_edges(target).filter(|other_incoming_edge| {
                                if other_incoming_edge.id == edge.id {
                                    return false;
                                }
                                match &other_incoming_edge.edge_type {
                                    EdgeType::SetAttribute { key, .. } if key == "src" => {
                                        other_incoming_edge.edge_timestamp.unwrap() <= set_attribute_timestamp
                                    }
                                    _ => false,
                                }
                            })
                            .map(|edge| edge.edge_timestamp.unwrap())
                            .collect::<Vec<_>>();

                        later_set_attribute_times.sort_unstable();
                        let next_timestamp = later_set_attribute_times.get(0);

                        self.outgoing_edges(target).filter(|edge| match edge.edge_type {
                                EdgeType::CrossDom {} => true,
                                _ => false,
                            }).filter(|edge| match &self.target_node(edge).node_type {
                                NodeType::DomRoot { url, .. } if url.as_deref() != Some("about:blank") => true,
                                NodeType::RemoteFrame { .. } => true,
                                _ => false,
                            }).filter(|edge|
                                edge.edge_timestamp.unwrap() >= set_attribute_timestamp &&
                                next_timestamp.map(|t| edge.edge_timestamp.unwrap() < *t).unwrap_or(true)
                            ).collect()
                    }
                    _ => vec![],
                }
            }
            EdgeType::DeleteAttribute { .. } => unimplemented!(),
            EdgeType::Binding { .. } => unimplemented!(),
            EdgeType::BindingEvent { .. } => unimplemented!(),
        }
    }

    /// Returns all actions that would not have occurred had the given action been omitted from the
    /// original graph.
    pub fn all_downstream_effects_of<'a>(&'a self, edge: &'a Edge) -> Vec<&'a Edge> {
        let mut edges_to_check = vec![edge];
        let mut already_checked = vec![];

        let original_edge = edge;

        while let Some(edge) = edges_to_check.pop() {
            let direct_effects = self.direct_downstream_effects_of(edge);
            if edge != original_edge {
                already_checked.push(edge);
            }

            direct_effects.into_iter().for_each(|edge|
                if !already_checked.contains(&edge) && edge != original_edge {
                    edges_to_check.push(edge);
                }
            );
        }

        already_checked
    }

    /// Returns all requests that would not have occurred had the given Request Start edge been
    /// omitted
    pub fn all_downstream_requests_nested<'a>(&'a self, edge: &'a Edge) -> Vec<DownstreamRequests> {
        let mut edges_to_check = vec![edge];
        let mut already_checked = vec![];
        let mut answer = vec![];

        let original_edge = edge;

        while let Some(edge) = edges_to_check.pop() {
            let direct_effects = self.direct_downstream_effects_of(edge);
            if edge != original_edge {
                already_checked.push(edge);
            }

            direct_effects.into_iter().for_each(|edge|
                if let EdgeType::RequestStart { request_id, request_type, .. } = &edge.edge_type {
                    let node = self.target_node(edge);
                    let url = match &node.node_type {
                        NodeType::Resource { url } => url,
                        _ => unreachable!()
                    };
                    let downstream_req = DownstreamRequests {
                        request_id: request_id.clone(),
                        request_type: request_type.clone(),
                        node_id: node.id,
                        url: url.to_string(),
                        children: self.all_downstream_requests_nested(edge)
                    };
                    answer.push(downstream_req)
                } else if !already_checked.contains(&edge) && edge != original_edge {
                    edges_to_check.push(edge);
                }
            );
        }
        answer
    }
}

fn get_domain(host: &str) -> String {
    if let "localhost" = host {
        return host.to_string();
    }
    let source_hostname = host;
    let source_domain = source_hostname.parse::<addr::DomainName>().expect("Source URL domain could not be parsed");
    let source_domain = &source_hostname[source_hostname.len() - source_domain.root().to_str().len()..];
    source_domain.to_string()
}
