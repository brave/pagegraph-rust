/// Represents the type of any PageGraph node, along with any associated type-specific data.
#[derive(Clone, PartialEq, Debug)]
pub enum NodeType {
    Extensions {},
    RemoteFrame {
        frame_id: String,
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
    FingerprintingV2Shield {},
}

#[derive(Clone, PartialEq, Debug)]
pub enum RequestType {
    Image,
    ScriptClassic,
    CSS,
    AJAX,
    Unknown,
}

impl From<&str> for RequestType {
    fn from(v: &str) -> Self {
        match v {
            "Image" => Self::Image,
            "ScriptClassic" => Self::ScriptClassic,
            "CSS" => Self::CSS,
            "AJAX" => Self::AJAX,
            "Unknown" => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

impl RequestType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::ScriptClassic => "script",
            Self::CSS => "stylesheet",
            Self::AJAX => "xhr",
            Self::Unknown => "unknown",
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
        pos: Option<usize>,
    },
    RequestComplete {
        resource_type: String,
        status: String,
        headers: String,
        size: isize,
        response_hash: Option<String>,
        request_id: usize,
    },
    RequestError {
        status: String,
        request_id: usize,
        headers: String,
        size: isize,
    },
    RequestStart {
        request_type: RequestType,
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
    ClearStorage { key: Option<String> },
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
