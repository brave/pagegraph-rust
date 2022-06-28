use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use std::convert::TryFrom;

use xml::reader::{ EventReader, XmlEvent };
use petgraph::graphmap::DiGraphMap;

use crate::{ graph, types };

/// Reads a PageGraph from a GraphML-formatted file.
pub fn read_from_file(file: &str) -> graph::PageGraph {
    let file = File::open(file).unwrap();
    let file = BufReader::new(file);

    let mut parser = EventReader::new(file);

    if let Ok(XmlEvent::StartDocument { .. }) = parser.next() {
        return parse_xml_document(&mut parser);
    } else {
        panic!("couldn't find start of document");
    }
}

fn parse_xml_document<R: std::io::Read>(parser: &mut EventReader<R>) -> graph::PageGraph {
    if let Ok(XmlEvent::StartElement { name, .. }) = parser.next() {
        if name.local_name == "graphml" {
            return parse_graphml(parser);
        } else {
            panic!("expected graphml element");
        }
    } else {
        panic!("could not find graphml element");
    }
}

/// For simple data items of the form `<local_name>This is the return value</local_name>`
fn parse_str_data<R: std::io::Read>(
    parser: &mut EventReader<R>,
    _attributes: Vec<xml::attribute::OwnedAttribute>,
    local_name: &str,
) -> String {
    let mut result = None;

    while let Ok(e) = parser.next() {
        match e {
            XmlEvent::EndElement { name } => {
                if name.local_name == local_name {
                    break
                }
            }
            XmlEvent::Characters(chars) => result = Some(chars),
            XmlEvent::Whitespace(_) => (),
            o => {panic!("Unexpected {:?} in `{}`", o, local_name)}
        }
    }

    return result.unwrap();
}

fn build_desc<R: std::io::Read>(
    parser: &mut EventReader<R>,
    _attributes: Vec<xml::attribute::OwnedAttribute>
) -> graph::PageGraphDescriptor {
    const STR_REP: &'static str = "desc";

    let mut version = None;
    let mut about = None;
    let mut url = None;
    let mut is_root = None;
    let mut frame_id = None;
    let mut time = None;

    while let Ok(e) = parser.next() {
        match e {
            XmlEvent::EndElement { name } => {
                if name.local_name == STR_REP {
                    break
                }
            }
            XmlEvent::StartElement { name, attributes, namespace: _ } => {
                let local_name = &name.local_name[..];
                match local_name {
                    "version" => version = Some(parse_str_data(parser, attributes, local_name)),
                    "about" => about = Some(parse_str_data(parser, attributes, local_name)),
                    "url" => url = Some(parse_str_data(parser, attributes, local_name)),
                    "is_root" => is_root = Some(parse_str_data(parser, attributes, local_name)),
                    "frame_id" => frame_id = Some(parse_str_data(parser, attributes, local_name)),
                    "time" => time = Some(build_time(parser, attributes)),
                    o => panic!("unexpected {:?} in `{}`", o, STR_REP),
                }
            }
            XmlEvent::Whitespace(_) => (),
            o => {panic!("Unexpected {:?} in `{}`", o, STR_REP)}
        }
    }

    graph::PageGraphDescriptor {
        version: version.unwrap(),
        about: about.unwrap(),
        url: url.unwrap(),
        is_root: is_root.unwrap().parse::<bool>().unwrap(),
        frame_id: graph::FrameId::try_from(frame_id.unwrap().as_str()).unwrap(),
        time: time.unwrap(),
    }
}

/// For the `time` element within `desc`.
fn build_time<R: std::io::Read>(
    parser: &mut EventReader<R>,
    _attributes: Vec<xml::attribute::OwnedAttribute>
) -> graph::PageGraphTime {
    const STR_REP: &str = "time";

    let mut start = None;
    let mut end = None;

    while let Ok(e) = parser.next() {
        match e {
            XmlEvent::EndElement { name } => {
                if name.local_name == STR_REP {
                    break
                }
            }
            XmlEvent::StartElement { name, attributes, namespace: _ } => {
                let local_name = &name.local_name[..];
                match local_name {
                    "start" => start = Some(parse_str_data(parser, attributes, local_name)),
                    "end" => end = Some(parse_str_data(parser, attributes, local_name)),
                    o => panic!("unexpected {:?} in `{}`", o, STR_REP),
                }
            }
            XmlEvent::Whitespace(_) => (),
            o => {panic!("Unexpected {:?} in `{}`", o, STR_REP)}
        }
    }

    graph::PageGraphTime {
        start: start.unwrap().parse::<u64>().unwrap(),
        end: end.unwrap().parse::<u64>().unwrap(),
    }
}

fn parse_graphml<R: std::io::Read>(parser: &mut EventReader<R>) -> graph::PageGraph {
    let mut desc = None;
    let mut node_items = HashMap::new();
    let mut edge_items = HashMap::new();
    while let Ok(e) = parser.next() {
        match e {
            XmlEvent::StartElement { name, attributes, namespace: _ } => {
                match &name.local_name[..] {
                    "key" => {
                        let (for_type, id, key) = build_key(parser, attributes);
                        match for_type {
                            KeyItemFor::Node => node_items.insert(id, key),
                            KeyItemFor::Edge => edge_items.insert(id, key),
                        };
                    }
                    "desc" => desc = Some(build_desc(parser, attributes)),
                    "graph" => {
                        break;
                    }
                    _ => println!("Unhandled local name: {}", name.local_name),
                }
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == "graphml" {
                    panic!("graphml ended without graph definition");
                } else {
                    panic!("unexpected end of element {}", name);
                }
            }
            XmlEvent::Whitespace(_) => (),
            o => {panic!("unexpected {:?} in `graphml`", o)}
        }
    }

    let key = KeyModel { node_items, edge_items };
    let graph = Some(build_graph(parser, &key, desc.expect("could not find desc")));

    while let Ok(e) = parser.next() {
        match e {
            XmlEvent::StartElement { name, attributes: _, namespace: _ } => {
                match &name.local_name[..] {
                    "key" => {
                        panic!("key item located after graph");
                    }
                    "graph" => {
                        panic!("more than one graph item not supported");
                    }
                    _ => println!("Unhandled local name: {}", name.local_name),
                }
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == "graphml" {
                    break
                }
            }
            XmlEvent::Whitespace(_) => (),
            o => {panic!("Unexpected {:?} in `graphml`", o)}
        }
    }

    graph.expect("could not find graph")
}

struct KeyModel {
    node_items: HashMap<String, KeyItem>,
    edge_items: HashMap<String, KeyItem>,
}

struct KeyItem {
    id: String,
    _attr_type: String,
}

enum KeyItemFor {
    Node,
    Edge,
}

impl TryFrom<&str> for KeyItemFor {
    type Error = ();

    fn try_from(v: &str) -> Result<Self, ()> {
        match v {
            "node" => Ok(Self::Node),
            "edge" => Ok(Self::Edge),
            _ => Err(())
        }
    }
}

fn build_key<R: std::io::Read>(
    parser: &mut EventReader<R>,
    attributes: Vec<xml::attribute::OwnedAttribute>
) -> (KeyItemFor, String, KeyItem) {
    let mut id = None;
    let mut for_type = None;
    let mut attr_name = None;
    let mut attr_type = None;
    for attribute in attributes {
        let name = attribute.name.local_name;
        match &name[..] {
            "id" => id = Some(attribute.value),
            "for" => for_type = Some(attribute.value),
            "attr.name" => attr_name = Some(attribute.value),
            "attr.type" => attr_type = Some(attribute.value),
            _ => panic!("Unexpected value in key: {}", &name),
        }
    }
    let key_item = KeyItem {
        id: id.expect("couldn't find `id` value on key"),
        _attr_type: attr_type.expect("couldn't find `attr.type` value on key"),
    };

    if let Ok(XmlEvent::EndElement { name }) = parser.next() {
        if &name.local_name != "key" {
            panic!("expected end of key element");
        }
    } else {
        panic!("could not find end of key element");
    }

    (
        KeyItemFor::try_from(&for_type.expect("couldn't find `for` value on key")[..])
            .expect("unexpected `for` value on key"),
        attr_name.expect("couldn't find `attr.name` value on key"),
        key_item,
    )
}

fn build_graph<R: std::io::Read>(parser: &mut EventReader<R>, key: &KeyModel, desc: graph::PageGraphDescriptor) -> graph::PageGraph {
    const STR_REP: &'static str = "graph";

    let mut edges = HashMap::new();
    let mut nodes = HashMap::new();
    let mut graph = DiGraphMap::<graph::NodeId, Vec<graph::EdgeId>>::new();

    while let Ok(e) = parser.next() {
        match e {
            XmlEvent::StartElement { name, attributes, namespace: _ } => {
                match &name.local_name[..] {
                    "node" => {
                        let node = build_node(parser, attributes, &key.node_items);
                        graph.add_node(node.id);
                        nodes.insert(node.id, node);
                    }
                    "edge" => {
                        let edge = build_edge(parser, attributes, &key.edge_items);
                        if let Some(concurrent_edges) = graph.edge_weight_mut(edge.source, edge.target) {
                            concurrent_edges.push(edge.id);
                        } else {
                            graph.add_edge(edge.source, edge.target, vec![edge.id]);
                        }
                        edges.insert(edge.id, edge);
                    }
                    _ => println!("Unhandled local name in {}: {}", STR_REP, name.local_name),
                }
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == STR_REP {
                    break
                }
            }
            XmlEvent::Whitespace(_) => (),
            o => {panic!("Unexpected {:?} in `{}`", o, STR_REP)}
        }
    }

    graph::PageGraph::new(desc, edges, nodes, graph)
}

fn build_edge<R: std::io::Read>(
    parser: &mut EventReader<R>,
    attributes: Vec<xml::attribute::OwnedAttribute>,
    key: &HashMap<String, KeyItem>
) -> graph::Edge {
    const STR_REP: &'static str = "edge";

    let mut id_value = None;
    let mut source_value = None;
    let mut target_value = None;
    let mut edge_type = None;
    let mut edge_timestamp = None;
    let mut data = HashMap::new();
    for attribute in attributes {
        let name = attribute.name.local_name;
        match &name[..] {
            "id" => id_value = Some(attribute.value
                    .trim_start_matches('e')
                    .parse::<usize>()
                    .expect("Parse edge id as usize")
                    .into()
                ),
            "source" => source_value = Some(attribute.value
                    .trim_start_matches('n')
                    .parse::<usize>()
                    .expect("Parse source node id as usize")
                    .into()
                ),
            "target" => target_value = Some(attribute.value
                    .trim_start_matches('n')
                    .parse::<usize>()
                    .expect("Parse target node id as usize")
                    .into()
                ),
            _ => panic!("Unexpected attribute in {}: {}", STR_REP, name),
        }
    }

    while let Ok(e) = parser.next() {
        match e {
            XmlEvent::StartElement { name, attributes, namespace: _ } => {
                match &name.local_name[..] {
                    DataItem::STR_REP => {
                        let data_item = DataItem::build_data(parser, attributes);
                        let contained = data_item.contained;
                        if key.get("edge type").unwrap().id == data_item.key {
                            edge_type = Some(contained.to_string());
                        } else if key.get("id").unwrap().id == data_item.key {
                            let edge_id: graph::EdgeId = contained.parse::<usize>()
                                .expect("parse edge id as usize")
                                .into();
                            if edge_id != id_value.unwrap() {
                                panic!("wrong edge id");
                            }
                        } else if key.get("timestamp").unwrap().id == data_item.key {
                            edge_timestamp = Some(if contained.contains('.') {
                                contained.trim_end_matches('0')
                                    .trim_end_matches('.')
                                    .parse::<isize>()
                                    .unwrap()
                                } else {
                                    contained.parse::<isize>()
                                        .unwrap_or_default()
                                });
                        } else {
                            data.insert(data_item.key, contained);
                        }
                    }
                    _ => println!("Unhandled local name in {}: {}", STR_REP, name.local_name),
                }
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == STR_REP {
                    break
                }
            }
            XmlEvent::Whitespace(_) => (),
            o => {panic!("Unexpected {:?} in `{}`", o, STR_REP)}
        }
    }

    let edge_type_attr = &edge_type.as_ref().expect("couldn't find `edge type` attr on node")[..];

    let edge_type = types::EdgeType::construct(edge_type_attr, &mut data, key);
    assert!(data.is_empty(), "extra data on edge {:?}: {:?}", edge_type, data);

    let id = id_value.expect("couldn't find `id` value on edge");
    let source = source_value.expect("couldn't find `source` value on edge");
    let target = target_value.expect("couldn't find `target` value on edge");

    graph::Edge {
        id,
        edge_type,
        edge_timestamp,
        source,
        target,
    }
}

fn build_node<R: std::io::Read>(
    parser: &mut EventReader<R>,
    attributes: Vec<xml::attribute::OwnedAttribute>,
    key: &HashMap<String, KeyItem>
) -> graph::Node {
    const STR_REP: &'static str = "node";

    let mut id_value = None;
    let mut node_type = None;
    let mut node_timestamp = None;
    let mut data = HashMap::new();
    for attribute in attributes {
        let name = attribute.name.local_name;
        match &name[..] {
            "id" => id_value = Some(attribute.value
                    .trim_start_matches('n')
                    .parse::<usize>()
                    .expect("Parse node id as usize")
                    .into()
                ),
            _ => panic!("Unexpected attribute in {}: {}", STR_REP, name),
        }
    }

    while let Ok(e) = parser.next() {
        match e {
            XmlEvent::StartElement { name, attributes, namespace: _ } => {
                match &name.local_name[..] {
                    DataItem::STR_REP => {
                        let data_item = DataItem::build_data(parser, attributes);
                        let contained = data_item.contained;
                        if key.get("node type").unwrap().id == data_item.key {
                            node_type = Some(contained.to_string());
                        } else if key.get("id").unwrap().id == data_item.key {
                            let node_id: graph::NodeId = contained.parse::<usize>()
                                .expect("parse node id as usize")
                                .into();
                            if node_id != id_value.unwrap() {
                                panic!("wrong node id");
                            }
                        } else if key.get("timestamp").unwrap().id == data_item.key {
                            node_timestamp = Some(if contained.contains('.') {
                                contained.trim_end_matches('0')
                                    .trim_end_matches('.')
                                    .parse::<isize>()
                                    .unwrap()
                                } else {
                                    contained.parse::<isize>()
                                        .unwrap_or_default()
                                });
                        } else {
                            data.insert(data_item.key, contained);
                        }
                    }
                    _ => println!("Unhandled local name in {}: {}", STR_REP, name.local_name),
                }
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == STR_REP {
                    break
                }
            }
            XmlEvent::Whitespace(_) => (),
            o => {panic!("Unexpected {:?} in `{}`", o, STR_REP)}
        }
    }

    let node_type_attr = &node_type.as_ref().expect("couldn't find `node type` attr on node")[..];

    let node_type = types::NodeType::construct(node_type_attr, &mut data, key);
    assert!(data.is_empty(), "extra data on node {:?}: {:?}", node_type, data);

    let id = id_value.expect("couldn't find `id` value on node");
    let node_timestamp = node_timestamp.expect("couldn't find `timestamp` attr on node");

    graph::Node {
        id,
        node_type,
        node_timestamp,
    }
}

/// Represents a `data` GraphML node, which provides attributes associated with a particular node
/// or edge.
#[derive(Debug, PartialEq)]
struct DataItem {
    key: String,
    contained: String,
}

impl DataItem {
    const STR_REP: &'static str = "data";

    fn build_data<R: std::io::Read>(
        parser: &mut EventReader<R>,
        attributes: Vec<xml::attribute::OwnedAttribute>
    ) -> Self {
        let mut key_value = None;
        let mut contained_value = None;

        for attribute in attributes {
            let name = attribute.name.local_name;
            match &name[..] {
                "key" => key_value = Some(attribute.value),
                _ => panic!("Unexpected attribute in {}: {}", Self::STR_REP, name),
            }
        }

        while let Ok(e) = parser.next() {
            match e {
                XmlEvent::EndElement { name } => {
                    if name.local_name == Self::STR_REP {
                        break
                    }
                }
                XmlEvent::Characters(c) => {
                    contained_value = Some(c);
                }
                XmlEvent::Whitespace(_) => (),
                o => {panic!("Unexpected {:?} in `{}`", o, Self::STR_REP)}
            }
        }

        Self {
            key: key_value.expect("couldn't find `key` value on data"),
            contained: contained_value.unwrap_or_default(),
        }
    }
}

/// Remove and return an attribute from an attribute map according to the key, if present
macro_rules! drain_opt_string_from {
    ( $attrs:ident, $key:ident, $attr:expr ) => {
        $attrs.remove(&$key.get($attr).expect(&format!("could not find `{}` in key", $attr)).id)
    };
}
/// Panic if the attribute string does not exist in the map
macro_rules! drain_string_from {
    ( $attrs:ident, $key:ident, $attr:expr ) => {
        drain_opt_string_from!($attrs, $key, $attr)
            .expect(&format!("attribute `{}` was not present", $attr))
    };
}
/// Panic if the attribute string cannot be parsed as a boolean value
macro_rules! drain_bool_from {
    ( $attrs:ident, $key:ident, $attr:expr ) => {
        drain_string_from!($attrs, $key, $attr)
            .to_ascii_lowercase()
            .parse::<bool>()
            .expect(&format!("could not parse attribute `{}` as bool", $attr))
    };
}
/// Panic if the optional attribute string cannot be parsed as an unsigned numeric value
macro_rules! drain_opt_usize_from {
    ( $attrs:ident, $key:ident, $attr:expr ) => {
        drain_opt_string_from!($attrs, $key, $attr)
            .map(|inner_data| inner_data
                .parse::<usize>()
                .expect(&format!("could not parse attribute `{}` as usize", $attr))
            )
    };
}
/// Panic if the attribute string cannot be parsed as an unsigned numeric value
macro_rules! drain_usize_from {
    ( $attrs:ident, $key:ident, $attr:expr ) => {
        {
            let value = drain_string_from!($attrs, $key, $attr);
            value
                .parse::<usize>()
                .expect(&format!("could not parse attribute `{}` as usize: `{}`", $attr, value))
        }
    };
}

/// Allows building this type from a type string and a set of associated attributes, each of which
/// correspond to intelligible string representations through a key.
///
/// Any attributes used will be drained from `attrs`.
trait KeyedAttrs {
    fn construct(type_str: &str, attrs: &mut HashMap<String, String>, key: &HashMap<String, KeyItem>) -> Self;
}

impl KeyedAttrs for types::NodeType {
    fn construct(type_str: &str, attrs: &mut HashMap<String, String>, key: &HashMap<String, KeyItem>) -> Self {
        macro_rules! drain_opt_string {
            ( $attr:expr ) => { drain_opt_string_from!(attrs, key, $attr) }
        }
        macro_rules! drain_string {
            ( $attr:expr ) => { drain_string_from!(attrs, key, $attr) }
        }
        macro_rules! drain_bool {
            ( $attr:expr ) => { drain_bool_from!(attrs, key, $attr) }
        }
        macro_rules! drain_usize {
            ( $attr:expr ) => { drain_usize_from!(attrs, key, $attr) }
        }

        match type_str {
            "extensions" => Self::Extensions {},
            "remote frame" => Self::RemoteFrame {
                frame_id: graph::FrameId::try_from(&drain_string!("frame id") as &str).unwrap()
            },
            "resource" => Self::Resource {
                url: drain_string!("url")
            },
            "ad filter" => Self::AdFilter {
                rule: drain_string!("rule")
            },
            "tracker filter" => Self::TrackerFilter,
            "fingerprinting filter" => Self::FingerprintingFilter,
            "web API" => Self::WebApi {
                method: drain_string!("method")
            },
            "JS builtin" => Self::JsBuiltin {
                method: drain_string!("method")
            },
            "HTML element" => Self::HtmlElement {
                tag_name: drain_string!("tag name"),
                is_deleted: drain_bool!("is deleted"),
                node_id: drain_usize!("node id"),
            },
            "text node" => Self::TextNode{
                text: drain_opt_string!("text"),
                is_deleted: drain_bool!("is deleted"),
                node_id: drain_usize!("node id"),
            },
            "DOM root" => Self::DomRoot {
                url: drain_opt_string!("url"),
                tag_name: drain_string!("tag name"),
                is_deleted: drain_bool!("is deleted"),
                node_id: drain_usize!("node id"),
            },
            "frame owner" => Self::FrameOwner {
                tag_name: drain_string!("tag name"),
                is_deleted: drain_bool!("is deleted"),
                node_id: drain_usize!("node id"),
            },
            "storage" => Self::Storage {},
            "local storage" => Self::LocalStorage {},
            "session storage" => Self::SessionStorage {},
            "cookie jar" => Self::CookieJar {},
            "script" => Self::Script {
                url: drain_opt_string!("url"),
                script_type: drain_string!("script type"),
                script_id: drain_usize!("script id"),
                source: drain_string!("source"),
            },
            "parser" => Self::Parser {},
            "Brave Shields" => Self::BraveShields {},
            "shieldsAds shield" => Self::AdsShield {},
            "trackers shield" => Self::TrackersShield {},
            "javascript shield" => Self::JavascriptShield {},
            "fingerprinting shield" => Self::FingerprintingShield {},
            "fingerprintingV2 shield" => Self::FingerprintingV2Shield {},
            "binding" => Self::Binding {
                binding: drain_string!("binding"),
                binding_type: drain_string!("binding type"),
            },
            "binding event" => Self::BindingEvent {
                binding_event: drain_string!("binding event"),
            },
            _ => panic!("Unknown node type `{}`", type_str),
        }
    }
}

impl KeyedAttrs for types::EdgeType {
    fn construct(type_str: &str, attrs: &mut HashMap<String, String>, key: &HashMap<String, KeyItem>) -> Self {
        macro_rules! drain_opt_string {
            ( $attr:expr ) => { drain_opt_string_from!(attrs, key, $attr) }
        }
        macro_rules! drain_string {
            ( $attr:expr ) => { drain_string_from!(attrs, key, $attr) }
        }
        macro_rules! drain_bool {
            ( $attr:expr ) => { drain_bool_from!(attrs, key, $attr) }
        }
        macro_rules! drain_opt_usize {
            ( $attr:expr ) => { drain_opt_usize_from!(attrs, key, $attr) }
        }
        macro_rules! drain_usize {
            ( $attr:expr ) => { drain_usize_from!(attrs, key, $attr) }
        }

        match type_str {
            "filter" => Self::Filter {},
            "structure" => Self::Structure {},
            "cross DOM" => Self::CrossDom {},
            "resource block" => Self::ResourceBlock {},
            "shield" => Self::Shield {},
            "text change" => Self::TextChange {},
            "remove node" => Self::RemoveNode {},
            "delete node" => Self::DeleteNode {},
            "insert node" => Self::InsertNode {
                parent: drain_usize!("parent"),
                before: drain_opt_usize!("before"),
            },
            "create node" => Self::CreateNode {},
            "js result" => Self::JsResult {
                value: drain_opt_string!("value"),
            },
            "js call" => Self::JsCall {
                args: drain_opt_string!("args"),
                script_position: drain_usize!("script position"),
            },
            "request complete" => Self::RequestComplete {
                resource_type: drain_string!("resource type"),
                status: drain_string!("status"),
                value: drain_opt_string!("value"),
                response_hash: drain_opt_string!("response hash"),
                request_id: drain_usize!("request id"),
                headers: drain_string!("headers"),
                size: drain_string!("size"),
            },
            "request error" => Self::RequestError {
                status: drain_string!("status"),
                request_id: drain_usize!("request id"),
                value: drain_opt_string!("value"),
                headers: drain_string!("headers"),
                size: drain_string!("size"),
            },
            "request start" => Self::RequestStart {
                request_type: crate::types::RequestType::from(&drain_string!("request type")[..]),
                status: drain_string!("status"),
                request_id: drain_usize!("request id"),
            },
            "request response" => Self::RequestResponse,
            "add event listener" => Self::AddEventListener {
                key: drain_string!("key"),
                event_listener_id: drain_usize!("event listener id"),
                script_id: drain_usize!("script id"),
            },
            "remove event listener" => Self::RemoveEventListener {
                key: drain_string!("key"),
                event_listener_id: drain_usize!("event listener id"),
                script_id: drain_usize!("script id"),
            },
            "event listener" => Self::EventListener{
                key: drain_string!("key"),
                event_listener_id: drain_usize!("event listener id"),
            },
            "storage set" => Self::StorageSet {
                key: drain_string!("key"),
                value: drain_opt_string!("value"),
            },
            "storage read result" => Self::StorageReadResult {
                key: drain_string!("key"),
                value: drain_opt_string!("value"),
            },
            "delete storage" => Self::DeleteStorage {
                key: drain_string!("key"),
            },
            "read storage call" => Self::ReadStorageCall {
                key: drain_string!("key"),
            },
            "clear storage" => Self::ClearStorage {
                key: drain_string!("key"),
            },
            "storage bucket" => Self::StorageBucket {},
            "execute from attribute" => Self::ExecuteFromAttribute {
                attr_name: drain_string!("attr name"),
            },
            "execute" => Self::Execute {},
            "set attribute" => Self::SetAttribute {
                key: drain_string!("key"),
                value: drain_opt_string!("value"),
                is_style: drain_bool!("is style"),
            },
            "delete attribute" => Self::DeleteAttribute {
                key: drain_string!("key"),
                is_style: drain_bool!("is style"),
            },
            "binding" => Self::Binding {},
            "binding event" => Self::BindingEvent {
                script_position: drain_usize!("script position"),
            },
            _ => panic!("Unknown edge type `{}`", type_str),
        }
    }
}
