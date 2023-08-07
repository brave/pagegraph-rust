use crate::graph::FrameId;

/// HtmlElementId represents the unsigned integer identifier that Blink uses
/// internally for each HTML element created during the execution of a Web page.
/// Note that values are unique (and monotonically increasing) for all
/// documents executing in the
/// [same process](https://developer.chrome.com/blog/site-isolation/).
/// This set of values are shared across HTML elements (e.g., `<a>`, `<img>`),
/// text nodes, and white space nodes.
type HtmlElementId = usize;

/// ScriptId represents the unsigned integer identifier V8 uses
/// for tracking each JavaScript code unit compiled and executed during
/// the execution of a page. Note that values are monotonically increasing for
/// all scripts executing, including scripts defined inline in `<script>`
/// tags, "classic" and "module" scripts fetched via the `<script>` element's
/// `src` property, or scripts otherwise passed to the JavaScript compiler
/// (e.g., `eval`, strings provided as the first argument to `setInterval`,
/// attributes like "onclick" defined in HTML elements, etc.).
type ScriptId = usize;

/// A string encoding a URL. May either be a full URL (protocol, host, port.
/// path, etc.) or a relative one, depending on the context in the graph.
type Url = String;

/// A string encoding the name of an HTML tag (e.g., `"a"` for an anchor tag,
/// or `"img"` for an image tag).
type HtmlTag = String;

/// A string encoding the name of an attribute on an HTML tag (e.g., `"href"`
/// for the target of an anchor tag, or `"src"` for the source URL of the image
/// presented in an image tag).
type HtmlAttr = String;

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
    JsBuiltin {
        method: String
    },
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
        tag_name: HtmlTag,
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
        url: Option<Url>,
        tag_name: HtmlTag,
        is_deleted: bool,
        node_id: HtmlElementId,
    },
    FrameOwner {
        tag_name: HtmlTag,
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
    /// The following edges record the results of [JavaScript code](NodeType::Script)
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
    /// The following edges record the results of [JavaScript code](NodeType::Script)
    /// receive when interacting with the storage area:
    /// - [`StorageReadResult`](EdgeType::StorageReadResult) records the
    ///   value returned to the script that read a value from the storage
    ///   area.
    SessionStorage {},
    /// Singleton node that represents the [`document.cookie`]()
    /// property read from, and written to, during the execution of the Web
    /// site. Note that this property *does not* encode cookies set or
    /// read through HTTP headers, only cookie activities from scripts.
    ///
    /// The following edges record [JavaScript code](NodeType::Script)
    /// reading and writing cookies during page execution:
    /// - [`StorageSet`](EdgeType::StorageSet): records when a script writes
    ///   assigns a value to the `document.cookie` property. Note though that
    ///   because of the how the JavaScript cookie API works, assigning a
    ///   value to the `document.cookie` property might actually delete
    ///   or modify a cookie in the site's cookie jar.
    /// - [`ReadStorageCall`](EdgeType::ReadStorageCall) records when a script
    ///   is reading the value of `document.cookie`.
    ///
    /// The following edges record value returned to [JavaScript code](NodeType::Script)
    /// reading the state of the `document.cookie` property.
    /// - [`StorageReadResult`](EdgeType::StorageReadResult) records the
    ///   value returned to the script that read the value of `document.cookie`.
    CookieJar {},
    /// Script nodes represent JavaScript code units compiled by V8
    /// and executed during the lifetime of the page. Script nodes
    /// encode any kind of script that can run during the page's execution
    /// (e.g., fetched or inlined "classic" scripts or module scripts,
    /// or eval'ed scripts).
    Script {
        /// The URL this script was fetched from, in the case that the script
        /// was fetched from a URL via a `<script>` element's `src` attribute,
        /// or a dynamically fetched module script.
        url: Option<Url>,
        /// The type of script being executed, either a "module" script
        /// or a "classic" script.
        script_type: String,
        /// The V8 identifier for this JavaScript code unit.
        script_id: ScriptId,
        /// The text of the script as passed to the v8 compiler.
        source: String,
    },
    /// Singleton node representing Blink parser, responsible for parsing
    /// HTML text and generating page elements.
    Parser {},
    Binding {
        binding: String,
        binding_type: String,
    },
    BindingEvent {
        binding_event: String,
    },
    RemoteFrame {
        frame_id: FrameId,
    },
    AdFilter {
        rule: String
    },
    TrackerFilter,  // TODO
    FingerprintingFilter,   // TODO
    Storage {},
    BraveShields {},
    AdsShield {},
    TrackersShield {},
    JavascriptShield {},
    FingerprintingShield {},
    FingerprintingV2Shield {},
    Extensions {},
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
/// Edges in PageGraph represent actions taken by some actor in the
/// page (e.g., a JavaScript code unit), being performed on some other element
/// in the page (e.g., a resource being fetched). Edges are outgoing from
/// the actor, and incoming to the actee.
#[derive(Clone, PartialEq, Debug)]
#[derive(serde::Serialize)]
pub enum EdgeType {
    CrossDom {},
    TextChange {},
    /// `RemoveNode` edges encode a HTML element being removed from the DOM
    /// tree.
    ///
    /// The actor node will be the [`Script`](NodeType::Script) node
    /// that is removing a HTML element from the document.
    ///
    /// The actee node will be the `HtmlElement`](NodeType::HtmlElement) node
    /// being removed from the document.
    RemoveNode {},
    /// `DeleteNode` edges encode a HTML element being deleted by JavaScript
    /// code. Note that this is a distinct action from merely removing an
    /// HTML element from a document (which is encoded with a
    /// [`RemoveNode`](EdgeType::RemoveNode) edge).
    ///
    /// The actor node will be the [`Script`](NodeType::Script) node
    /// that is delete a HTML element.
    ///
    /// The actee node will be the `HtmlElement`](NodeType::HtmlElement) node
    /// being deleted.
    DeleteNode {},
    /// `InsertNode` edges encode a HTML element being inserted into a DOM
    /// tree.
    ///
    /// The actor node will either be the [`Parser`](NodeType::Parser)
    /// (indicating that the element was inserted into the document because
    /// of text being parsed, most often from the initial HTML document)
    /// or a [`Script`](NodeType::Script) node (indicating that the element
    /// was inserted into the document dynamically).
    ///
    /// The actee node will be a [`HtmlElement`](NodeType::HtmlElement) node
    /// depicting the HTML element being inserted into the document.
    InsertNode {
        /// The identifier of the DOM element the actee
        /// [`HtmlElement`](NodeType::HtmlElement) node is being inserted
        /// beneath in the document.
        parent: HtmlElementId,
        /// The identifier of the prior sibling DOM element the actee
        /// [`HtmlElement`](NodeType::HtmlElement) node is being inserted
        /// before in the document. If this value is not present, it indicates
        /// that the actee node was the first child of the parent node at
        /// insertion time,
        before: Option<HtmlElementId>,
    },
    /// `CreateNode` edges encode that an HTML element that was created during
    /// the execution of the page.
    ///
    /// The actor node will either be the [`Parser`](NodeType::Parser)
    /// (indicating that the element was created because it was defined in
    /// text parsed by the blink parser) or a [`Script`](NodeType::Script) node
    /// (indicating that the element was dynamically created by a JavaScript
    /// code unit (e.g., `document.createElement`).
    ///
    /// The actee node will be a [`HtmlElement`](NodeType::HtmlElement) node
    /// depicting the HTML element that was created.
    CreateNode {},
    /// `JsResult` edges encode a value being returned from a property read
    /// or a function call in JavaScript code.
    ///
    /// The actor node will be either a [`WebApi`](NodeType::WebApi) node
    /// (representing the WebAPI method or property that was called) or
    /// a [`JsBuiltin`](NodeType::JsBuiltin) node (representing
    /// an instrumented method or function thats defined as part of
    /// ECMAScript).
    ///
    /// The actee node will be a [`Script`](NodeType::Script) node
    /// representing the JavaScript code unit that the value is being
    /// returned to.
    JsResult {
        /// The value being returned from an API to a JavaScript code unit.
        /// Note that this will not be present for APIs that do not return
        /// a value when called, such as `addEventListener`).
        value: Option<String>
    },
    /// `JsCall` edges encode a JavaScript function/method being called
    /// by JavaScript code.
    ///
    /// The actor node will be [`Script`](NodeType::Script) node, representing
    /// the JavaScript code unit calling the property, function, or method.
    ///
    /// The actee node will be either a [`WebApi`](NodeType::WebApi) node
    /// (representing the WebAPI method or property being called) or
    /// a [`JsBuiltin`](NodeType::JsBuiltin) node (representing
    /// an instrumented method or function thats defined as part of
    /// ECMAScript being called).
    JsCall {
        /// An serialized version of any arguments provided when the method or
        /// function being called (if any).
        args: Option<String>,
        /// The character offset in the JavaScript text where this JavaScript
        /// call occurred.
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
        script_id: ScriptId,
    },
    RemoveEventListener {
        key: String,
        event_listener_id: usize,
        script_id: ScriptId,
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
        key: String
    },
    ReadStorageCall {
        key: String
    },
    ClearStorage {
        key: String
    },
    ExecuteFromAttribute {
        attr_name: HtmlAttr
    },
    Execute {},
    /// `SetAttribute` edges encode JavaScript code setting an attribute
    /// on a HTML element.
    ///
    /// The actor node will be [`Script`](NodeType::Script) node, representing
    /// the JavaScript code setting the attribute.
    ///
    /// The actee node will be a [`HtmlElement`](NodeType::HtmlElement) node,
    /// representing the HTML element that is having an attribute set on it.
    SetAttribute {
        /// The name of the HTML attribute being set (e.g., `height`,
        /// `href`, `src`).
        key: HtmlAttr,
        /// The value being assigned to the HTML attribute (if any).
        value: Option<String>,
        /// If the attribute being set is part of the the element's
        /// CSS style definition.
        is_style: bool,
    },
    /// `DeleteAttribute` edges encode JavaScript code deleting an attribute
    /// from a HTML element.
    ///
    /// The actor node will be [`Script`](NodeType::Script) node, representing
    /// the JavaScript code deleting the attribute.
    ///
    /// The actee node will be a [`HtmlElement`](NodeType::HtmlElement) node,
    /// representing the HTML element that is having the attribute deleted.
    DeleteAttribute {
        /// The name of the HTML attribute being deleted (e.g., `height`,
        /// `href`, `src`).
        key: HtmlAttr,
        /// If the attribute being deleted is part of the the element's
        /// CSS style definition.
        is_style: bool,
    },
    Binding {},
    BindingEvent {
        script_position: usize,
    },
    Filter {},
    Structure {},
    Shield {},
    ResourceBlock {},
    StorageBucket {},
}
