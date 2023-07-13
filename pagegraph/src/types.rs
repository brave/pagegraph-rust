use crate::graph::FrameId;

/// HtmlElementId represents the int identifier that Blink uses internally
/// for each HTML element created during the execution of a Web page.
/// Note that values are unique (and monotonically increasing) for all
/// documents executing in the
/// [same process](https://developer.chrome.com/blog/site-isolation/).
/// This set of values are shared across HTML elements (e.g., `<a>`, `<img>`),
/// text nodes, and white space nodes.
type HtmlElementId = usize;

/// Represents the type of any PageGraph node, along with any associated type-specific data.
/// Nodes in PageGraph (mostly) represent either Actors (things that do things)
/// or Actees (things that have things done to them).
///
/// For example, if JavaScript code creates a HTML element and injects it into
/// a document, that would be represented in PageGraph with three nodes:
///
/// 1. a node representing the JavaScript code unit
/// 2. a node representing the HTML element that was created, and
/// 3. a third node representing the existing HTML element the just created
///    HTML element is inserted below in the DOM.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub enum NodeType {
    /// Resource nodes record URLs that are requested from network. Each
    /// URL requested is represented with its own Resource node. Each
    /// request is denoted with a [`RequestStart`](EdgeType::RequestStart) edge, and each
    /// response is denoted with either a [`RequestComplete`](EdgeType::RequestComplete) or
    /// [`RequestError`](EdgeType::RequestError) edge (which record whether the request to the
    /// URL succeeded, and if so, what was returned).
    Resource {
        /// The URL represented by this node.
        url: String
    },
    /// WebApi nodes represent [Web APIs](https://developer.mozilla.org/en-US/docs/Web/API)
    /// provided by the browser that JavaScript code can call. There will be at
    /// most one WebApi node for each Web API called during the execution
    /// of the page.
    ///
    /// Each call to the method represented by this node is encoded
    /// with the following:
    /// 1. a [`Script`](NodeType::Script) node, recording the JavaScript code unit
    ///    calling the Web API.
    /// 2. an outgoing [`JsCall`](EdgeType::JsCall) edge, recording the arguments
    ///    provided when calling the API), if any.
    /// 3. a [`WebApi`](NodeType::WebApi) node , recording the Web API method being
    ///    called
    /// 4. a returning (incoming) [`JsResult`](EdgeType::JsResult) edge, recording the
    ///    value returned by the WebApi, if any
    ///
    /// Note that for performance reasons, only Web APIs specified during a
    /// Brave build are recorded in PageGraph. The list of APIs Brave
    /// records by default in Nightly and Beta builds can be found
    /// [in the Brave source code](https://github.com/brave/brave-core/blob/master/chromium_src/third_party/blink/renderer/bindings/scripts/bind_gen/interface.py#L20).
    /// Note further that no WebAPIs are recorded in PageGraph in Stable Brave
    /// builds. If you want to record other WebApis in PageGraph, you
    /// should modify [the PageGraph interface.py script](https://github.com/brave/brave-core/blob/master/chromium_src/third_party/blink/renderer/bindings/scripts/bind_gen/interface.py#L20)
    /// and then rebuild Brave.
    WebApi {
        /// The path to the Web API function this node represents.
        /// For example, a value of
        /// [`Performance.now`](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now)
        /// denotes the JavaScript method provided in most browsers at
        /// `window.performance.now()`.
        method: String
    },
    JsBuiltin { method: String },
    /// HTMLElement nodes represent the elements that make up the structure
    /// of a Web page. They map to things like `<a>`, `<img>`, `<div>`, etc.
    /// PageGraph creates one for each HTML element that exists at any point
    /// during the lifetime of the web page (even those that are not
    /// inserted into the DOM).
    HtmlElement {
        /// Stores the tag name of the HTML element, similar to the
        /// [`tagName`](https://developer.mozilla.org/en-US/docs/Web/API/Element/tagName)
        /// JavaScript attribute. For an image tag, this value will be "img",
        /// for a unordered list element this value will be "ul", etc.
        tag_name: String,
        /// Records whether the node is alive (and not garbage collected)
        /// at the point in time when the PageGraph document was serialized.
        is_deleted: bool,
        /// The integer identifier Blink assigns to each element in the DOM
        /// of a web page. Will be unique for all
        /// [`HtmlElements`](NodeType::HtmlElement),
        /// [TextNodes](NodeType.TextNode), and [DomRoots](NodeType.DomRoot)
        /// within the same process (in terms of process isolation).
        ///
        /// This value referenced from the attributes of several actions (i.e.,
        /// [events][EdgeType]) recorded in PageGraph. For example, when a
        /// a [JavaScript unit](NodeType::Script) [inserts](EdgeType::InsertNode)
        /// a [`HtmlElement`](NodeType::HtmlElement) into a document, the
        /// [`InsertNode`](EdgeType::InsertNode) edge's
        /// [`parent`](EdgeType::InsertNode::parent) attribute records the
        /// [`node_id`](NodeType::HtmlElement::node_id) of the HtmlElement the
        /// node was inserted below.
        node_id: HtmlElementId,
    },
    /// TextNode nodes represent [text nodes](https://developer.mozilla.org/en-US/docs/Web/API/Text)
    /// in the DOM tree. PageGraph uses this node type to also represent
    /// white space, so (unfortunately) all text nodes don't necessarily
    /// include text; some are just white space.
    TextNode {
        /// The text string, if any, in the DOM tree, represented by this node.
        text: Option<String>,
        /// Records whether the node is alive (and not garbage collected)
        /// at the point in time when the PageGraph document was serialized.
        is_deleted: bool,
        /// The integer identifier Blink assigns to each element in the DOM
        /// of a web page. Will be unique for all
        /// [`HtmlElements`](NodeType::HtmlElement),
        /// [TextNodes](NodeType.TextNode), and [DomRoots](NodeType.DomRoot)
        /// within the same process (in terms of process isolation).
        ///
        /// This value referenced from the attributes of several actions (i.e.,
        /// [events][EdgeType]) recorded in PageGraph. For example, when a
        /// a [JavaScript unit](NodeType::Script) [inserts](EdgeType::InsertNode)
        /// a [text](NodeType::TextNode) into a document, the
        /// [`InsertNode`](EdgeType::InsertNode) edge's
        /// [`parent`](EdgeType::InsertNode::parent) attribute records the
        /// [`node_id`](NodeType::TextNode::node_id) of the TextNode the
        /// node was inserted below.
        node_id: HtmlElementId,
    },
    DomRoot {
        url: Option<String>,
        tag_name: String,
        is_deleted: bool,
        node_id: HtmlElementId,
    },
    FrameOwner {
        tag_name: String,
        is_deleted: bool,
        node_id: HtmlElementId,
    },
    /// Singleton node that represents the [`window.localStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
    /// storage area read from, and written to, during the execution of the Web
    /// site.
    ///
    /// Note that this node does not record what values are in the storage
    /// area at any given point. To observe what keys are written to and read
    /// from local storage, consider instead the incoming and outgoing edges to
    /// this node.
    ///
    /// The following edges record [JavaScript code](NodeType::Script)
    /// interacting with the storage area:
    /// - [`StorageSet`](EdgeType::StorageSet): records when a script writes
    ///   a value to the storage area. This maps to calling [`setItem`](https://developer.mozilla.org/en-US/docs/Web/API/Storage/setItem)
    ///   (or equivalent) on the storage area.
    /// - [`ReadStorageCall`](EdgeType::ReadStorageCall) records when a script
    ///   attempts to read a value from the storage area. This corresponds to
    ///   a script calling [`getItem`](https://developer.mozilla.org/en-US/docs/Web/API/Storage/getItem)
    ///   (or equivalent) on the storage area.
    /// - [`DeleteStorage`](EdgeType::DeleteStorage) records when a script
    ///   deletes a key from the storage area. This corresponds to a
    ///   script calling [`removeItem`](https://developer.mozilla.org/en-US/docs/Web/API/Storage/removeItem)
    ///   on the storage area.
    /// - [`ClearStorage`](EdgeType::ClearStorage) records when a script
    ///   clears all values from the storage area. This corresponds to a script
    ///   calling [`clear`](https://developer.mozilla.org/en-US/docs/Web/API/Storage/clear)
    ///   on the storage area.
    ///
    /// The following edges record the results [JavaScript code](NodeType::Script)
    /// receive when interacting with the storage area:
    /// - [`StorageReadResult`](EdgeType::StorageReadResult) records the
    ///   value returned to the script that read a value from the storage
    ///   area.
    LocalStorage {},
    /// Singleton node that represents the [`window.sessionStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/sessionStorage)
    /// storage area read from, and written to, during the execution of the Web
    /// site.
    ///
    /// Note that this node does not record what values are in the storage
    /// area at any given point. To observe what keys are written to and read
    /// from session storage, consider instead the incoming and outgoing edges to
    /// this node.
    ///
    /// The following edges record [JavaScript code](NodeType::Script)
    /// interacting with the storage area:
    /// - [`StorageSet`](EdgeType::StorageSet): records when a script writes
    ///   a value to the storage area. This maps to calling [`setItem`](https://developer.mozilla.org/en-US/docs/Web/API/Storage/setItem)
    ///   (or equivalent) on the storage area.
    /// - [`ReadStorageCall`](EdgeType::ReadStorageCall) records when a script
    ///   attempts to read a value from the storage area. This corresponds to
    ///   a script calling [`getItem`](https://developer.mozilla.org/en-US/docs/Web/API/Storage/getItem)
    ///   (or equivalent) on the storage area.
    /// - [`DeleteStorage`](EdgeType::DeleteStorage) records when a script
    ///   deletes a key from the storage area. This corresponds to a
    ///   script calling [`removeItem`](https://developer.mozilla.org/en-US/docs/Web/API/Storage/removeItem)
    ///   on the storage area.
    /// - [`ClearStorage`](EdgeType::ClearStorage) records when a script
    ///   clears all values from the storage area. This corresponds to a script
    ///   calling [`clear`](https://developer.mozilla.org/en-US/docs/Web/API/Storage/clear)
    ///   on the storage area.
    ///
    /// The following edges record the results [JavaScript code](NodeType::Script)
    /// receive when interacting with the storage area:
    /// - [`StorageReadResult`](EdgeType::StorageReadResult) records the
    ///   value returned to the script that read a value from the storage
    ///   area.
    SessionStorage {},
    CookieJar {},
    Script {
        url: Option<String>,
        script_type: String,
        script_id: usize,
        source: String,
    },
    Parser {},
    BraveShields {},
    AdsShield {},
    TrackersShield {},
    JavascriptShield {},
    FingerprintingShield {},
    FingerprintingV2Shield {},
    Binding {
        binding: String,
        binding_type: String,
    },
    BindingEvent {
        binding_event: String,
    },
    Extensions {},
    RemoteFrame {
        frame_id: FrameId,
    },
    AdFilter { rule: String },
    TrackerFilter,  // TODO
    FingerprintingFilter,   // TODO
    Storage {},
}

#[derive(Clone, PartialEq, Debug)]
#[derive(serde::Serialize)]
pub enum RequestType {
    Image,
    Script,
    CSS,
    AJAX,
    Unknown,
}

impl From<&str> for RequestType {
    fn from(v: &str) -> Self {
        match v {
            "Image" => Self::Image,
            "Script" => Self::Script,
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
            Self::Script => "script",
            Self::CSS => "stylesheet",
            Self::AJAX => "xhr",
            Self::Unknown => "unknown",
        }
    }
}

/// Represents the type of any PageGraph edge, along with any associated type-specific data.
#[derive(Clone, PartialEq, Debug)]
#[derive(serde::Serialize)]
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
    JsResult { value: Option<String> },
    JsCall {
        args: Option<String>,
        script_position: usize,
    },
    RequestComplete {
        resource_type: String,
        status: String,
        value: Option<String>,
        response_hash: Option<String>,
        request_id: usize,
        headers: String,
        size: String,
    },
    RequestError {
        status: String,
        request_id: usize,
        value: Option<String>,
        headers: String,
        size: String,
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
    DeleteStorage { key: String },
    ReadStorageCall { key: String },
    ClearStorage { key: String },
    StorageBucket {},
    ExecuteFromAttribute { attr_name: String },
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
    Binding {},
    BindingEvent {
        script_position: usize,
    },
}
