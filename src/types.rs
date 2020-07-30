use std::fmt;

/// Represents the type of any PageGraph node, along with any associated type-specific data.
#[derive(Clone, PartialEq, Debug)]
pub enum NodeType {
    Extensions {},
    RemoteFrame {
        url: String,
    },
    Resource {
        url: String,
    },
    AdFilter {
        rule: String,
    },
    TrackerFilter,        // TODO
    FingerprintingFilter, // TODO
    WebApi {
        method: String,
    },
    JsBuiltin {
        method: String,
    },
    HtmlElement {
        tag_name: String,
        is_deleted: bool,
        node_id: usize,
    },
    TextNode {
        text: Option<String>,
        is_deleted: bool,
        node_id: usize,
    },
    DomRoot {
        url: Option<String>,
        tag_name: String,
        is_deleted: bool,
        node_id: usize,
    },
    FrameOwner {
        tag_name: String,
        is_deleted: bool,
        node_id: usize,
    },
    Storage {},
    LocalStorage {},
    SessionStorage {},
    CookieJar {},
    Script {
        url: Option<String>,
        script_type: String,
        script_id: usize,
    },
    Parser {},
    BraveShields {},
    AdsShield {},
    TrackersShield {},
    JavascriptShield {},
    FingerprintingShield {},
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.NodeType {
          HtmlElement => {
            // write!(f, "(HtmlElement: <{}, {})", self.x, self.y)
          }
        }
    }
}

/// Represents the type of any PageGraph edge, along with any associated type-specific data.
#[derive(Clone, PartialEq, Debug)]
pub enum EdgeType {
    Filter {},
    Structure {},
    CrossDom {},
    ResourceBlock {},
    Shield {},
    TextChange {},
    RemoveNode {},
    DeleteNode {},
    InsertNode {
        parent: usize,
        before: Option<usize>,
    },
    CreateNode {},
    JsResult {
        value: Option<String>,
    },
    JsCall {
        args: Option<String>,
    },
    RequestComplete {
        resource_type: String,
        status: String,
        value: String,
        response_hash: Option<String>,
        request_id: usize,
    },
    RequestError {
        status: String,
        request_id: usize,
        value: String,
    },
    RequestStart {
        request_type: String,
        status: String,
        request_id: usize,
    },
    RequestResponse, // TODO
    AddEventListener {
        key: String,
        event_listener_id: usize,
        script_id: usize,
    },
    RemoveEventListener {
        key: String,
        event_listener_id: usize,
        script_id: usize,
    },
    EventListener {
        key: String,
        event_listener_id: usize,
    },
    StorageSet {
        key: String,
        value: Option<String>,
    },
    StorageReadResult {
        key: String,
        value: Option<String>,
    },
    DeleteStorage {
        key: String,
    },
    ReadStorageCall {
        key: String,
    },
    ClearStorage, // TODO
    StorageBucket {},
    ExecuteFromAttribute {
        attr_name: String,
    },
    Execute {},
    SetAttribute {
        key: String,
        value: Option<String>,
        is_style: bool,
    },
    DeleteAttribute {
        key: String,
        is_style: bool,
    },
}
