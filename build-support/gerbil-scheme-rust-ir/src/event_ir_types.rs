//! Versioned typed event IR accepted by the Scheme-to-Rust event compiler.

use serde::Deserialize;

/// Versioned wire contract for source-backed event functions.
pub const EVENT_FUNCTION_IR_SCHEMA: &str = "gerbil-scheme-rust.event-function-ir.v1";

/// One Scheme-authored event procedure with line-local and final transitions.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventFunctionIr {
    /// Exact supported wire schema.
    pub schema: String,
    /// Public generated Rust function name.
    pub name: String,
    /// Declared root syntax-kind index.
    pub root_kind: u16,
    /// Digest of the source-owned parser algorithm.
    pub parser_digest: String,
    /// Initial state declarations, evaluated before the line fold.
    pub initial: Vec<EventStatementIr>,
    /// One transition for each source line.
    pub line: Vec<EventStatementIr>,
    /// Final transitions before the root closes.
    pub finish: Vec<EventStatementIr>,
    /// Bounded source-local event procedures owned by the Scheme parser.
    #[serde(default)]
    pub helpers: Vec<EventHelperIr>,
}

/// A fresh-state event procedure reused at multiple source spans.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventHelperIr {
    pub name: String,
    pub initial: Vec<EventStatementIr>,
    pub body: Vec<EventStatementIr>,
}

/// Closed, auditable side effects permitted in an event procedure.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventStatementIr {
    /// Declare one bounded mutable boolean state slot.
    LetBool { name: String, value: bool },
    /// Declare one unsigned parser state slot.
    LetUsize { name: String, value: usize },
    /// Declare a nesting stack of unsigned levels.
    LetUsizeStack { name: String },
    /// Update a previously declared state slot.
    SetBool {
        name: String,
        value: EventPredicateIr,
    },
    /// Update a previously declared unsigned state slot.
    SetUsize { name: String, value: EventUsizeIr },
    /// Close all nested nodes at or above a new level.
    CloseThroughLevel { stack: String, level: EventUsizeIr },
    /// Open one nested node at a declared level.
    OpenLevel {
        stack: String,
        level: EventUsizeIr,
        syntax_kind: u16,
    },
    /// Close every remaining node in a nesting stack.
    CloseAllLevels { stack: String },
    /// Emit an opening Rowan node event.
    StartNode { syntax_kind: u16 },
    /// Emit a source-backed token for the current line.
    Token {
        syntax_kind: u16,
        start: EventOffsetIr,
        end: EventOffsetIr,
    },
    /// Emit a closing Rowan node event.
    FinishNode,
    /// Choose one statically bounded transition branch.
    If {
        condition: EventPredicateIr,
        consequent: Vec<Self>,
        alternate: Vec<Self>,
    },
    /// Iterate a bounded source-line byte range; the index is source-backed.
    ForLineBytes {
        index: String,
        from: EventOffsetIr,
        until: EventOffsetIr,
        body: Vec<Self>,
    },
    /// Rebind line-scoped parsing primitives to a checked UTF-8 source span.
    WithSourceBounds {
        from: EventOffsetIr,
        until: EventOffsetIr,
        body: Vec<Self>,
    },
    /// Execute one Scheme-owned helper at a checked source span.
    CallSourceHelper {
        name: String,
        from: EventOffsetIr,
        until: EventOffsetIr,
    },
    /// Read a source-line list marker into typed state slots.
    ScanListMarker { marker: EventListMarkerIr },
    /// Push an unsigned frame owned by the Scheme transition.
    PushFrame { stack: String, value: EventUsizeIr },
    /// Pop frames while a Scheme predicate holds, emitting a fixed close arity.
    CloseFramesWhile {
        stack: String,
        condition: EventPredicateIr,
        finish_count: u8,
    },
    /// Close every frame, emitting a fixed number of node closes per frame.
    CloseAllFrames { stack: String, finish_count: u8 },
}

/// Language-declared list marker shape and typed event-state destinations.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventListMarkerIr {
    /// Unordered one-byte marker alternatives.
    pub unordered: String,
    /// Whether decimal and alphabetic ordered bullets are admitted.
    pub ordered: bool,
    /// Positive tab stop width used for indentation columns.
    pub tab_width: usize,
    /// Boolean slot for a successfully recognized marker.
    pub present: String,
    /// Unsigned indentation column slot.
    pub column: String,
    /// Boolean slot for ordered/unordered shape.
    pub ordered_slot: String,
    /// Source-backed bullet-start slot.
    pub bullet_start: String,
    /// Source-backed bullet-end slot.
    pub bullet_end: String,
    /// Source-backed content-start slot.
    pub content_start: String,
}

/// Source byte offsets available in a line transition.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum EventOffsetIr {
    /// An existing current-line boundary.
    Boundary(EventBoundaryIr),
    /// A statically bounded source-line byte calculation.
    Computed(EventComputedOffsetIr),
}

/// Current source-line boundaries.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventBoundaryIr {
    /// Inclusive current-line start.
    Start,
    /// Exclusive current-line end.
    End,
}

/// Bounded source-line byte offsets without arbitrary Rust expressions.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventComputedOffsetIr {
    /// End of a declared literal prefix, clamped to the current line.
    LinePrefixEnd { value: String },
    /// Skip spaces and tabs from a source-backed offset.
    LineSkipHorizontal { from: Box<EventOffsetIr> },
    /// Scan the next whitespace-delimited word from a source-backed offset.
    LineScanWord { from: Box<EventOffsetIr> },
    /// Scan an ASCII identifier composed of letters, digits, underscore and hyphen.
    LineScanKey { from: Box<EventOffsetIr> },
    /// Scan non-whitespace bytes up to a declared delimiter within one source line.
    LineScanNonspaceUntil {
        from: Box<EventOffsetIr>,
        delimiter: u8,
    },
    /// Scan to a declared byte delimiter, including intervening whitespace.
    LineScanUntil {
        from: Box<EventOffsetIr>,
        delimiter: u8,
    },
    /// Scan to the next CR or LF inside the current checked source span.
    LinePhysicalEnd { from: Box<EventOffsetIr> },
    /// Move one byte forward without passing the current line boundary.
    LineStep { from: Box<EventOffsetIr> },
    /// Remove trailing ASCII whitespace from the current source line.
    LineTrimEnd,
    /// Trim trailing whitespace without crossing a source-backed value start.
    LineTrimEndFrom { from: Box<EventOffsetIr> },
    /// End before a final CR/LF, preserving horizontal source whitespace.
    LineContentEnd,
    /// A byte index bound by a surrounding source-line iteration.
    LineIndex { name: String },
    /// A named unsigned state containing a current source-line offset.
    StateOffset { name: String },
    /// End of a checked leading marker run; reuses the line's cached level.
    LineMarkerEnd { marker: u8, separator: u8 },
}

/// Closed unsigned-value vocabulary for contextual line transitions.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventUsizeIr {
    /// A literal nonnegative value.
    Usize { value: usize },
    /// A named unsigned parser state slot.
    State { name: String },
    /// Count repeated leading marker bytes only when followed by a separator.
    LineMarkerLevel { marker: u8, separator: u8 },
    /// Promote a checked source-backed offset to an unsigned state value.
    Offset { value: EventOffsetIr },
    /// Tab-aware leading indentation width of the current source line.
    LineIndentColumn { tab_width: usize },
    /// Top of a nonempty unsigned frame stack; zero when empty.
    StackTop { stack: String },
    /// Checked unsigned arithmetic for encoded frame values.
    Add { left: Box<Self>, right: Box<Self> },
    /// Checked multiplication for encoded frame values.
    Multiply { left: Box<Self>, right: Box<Self> },
    /// Bounded positive-divisor quotient for encoded frame values.
    Divide { left: Box<Self>, right: Box<Self> },
}

/// Closed predicate vocabulary; it has no arbitrary Rust expression node.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventPredicateIr {
    /// A literal truth value.
    Bool { value: bool },
    /// Read one named state slot.
    State { name: String },
    /// Test whether an unsigned expression is positive.
    UsizePositive { value: EventUsizeIr },
    /// Compare two typed unsigned parser values.
    UsizeEqual {
        left: EventUsizeIr,
        right: EventUsizeIr,
    },
    /// Distinguish two typed unsigned parser values.
    UsizeNotEqual {
        left: EventUsizeIr,
        right: EventUsizeIr,
    },
    /// Compare two unsigned parser values.
    UsizeGreater {
        left: EventUsizeIr,
        right: EventUsizeIr,
    },
    /// Compare two source-backed byte offsets.
    OffsetLess {
        left: EventOffsetIr,
        right: EventOffsetIr,
    },
    /// Test whether a named unsigned frame stack contains a frame.
    StackNonempty { stack: String },
    /// Compare the current source line's prefix.
    LineStartsWith { value: String },
    /// Compare a source-line prefix using ASCII-insensitive syntax matching.
    LineStartsWithAsciiCaseInsensitive { value: String },
    /// Match an ASCII-insensitive prefix only at a horizontal or line boundary.
    LinePrefixBoundaryAsciiCaseInsensitive { value: String },
    /// Match a whole line marker with only trailing ASCII whitespace.
    LineMarkerAsciiCaseInsensitive { value: String },
    /// Search later source lines for a whole marker before a heading or parent marker.
    FutureLineMarkerBeforeBoundary {
        target: String,
        stop: String,
        heading_marker: u8,
        heading_separator: u8,
        indent: bool,
        stop_at_heading: bool,
        #[serde(default)]
        body_key_marker: u8,
    },
    /// Treat spaces, tabs, and line endings as a blank source line.
    LineBlank,
    /// A declared prefix is followed by one nonempty whitespace-delimited word.
    LineHasWordAfterPrefix { value: String },
    /// A prefix is followed by a nonempty ASCII key and a colon.
    LineHasKeyAfterPrefix { value: String },
    /// Compare one source-line byte at a bounded source-backed offset.
    LineByteEqual { at: EventOffsetIr, value: u8 },
    /// All bytes in one bounded slice belong to an explicit byte set.
    LineBytesAllIn {
        from: EventOffsetIr,
        until: EventOffsetIr,
        values: Vec<u8>,
    },
    /// At least one byte in one bounded slice belongs to an explicit byte set.
    LineBytesAnyIn {
        from: EventOffsetIr,
        until: EventOffsetIr,
        values: Vec<u8>,
    },
    /// Match a bounded source-line byte slice against a sorted static name set.
    LineBytesInSet {
        from: EventOffsetIr,
        until: EventOffsetIr,
        values: Vec<String>,
    },
    /// Boolean negation.
    Not { value: Box<Self> },
    /// Short-circuit conjunction.
    And { left: Box<Self>, right: Box<Self> },
    /// Short-circuit disjunction.
    Or { left: Box<Self>, right: Box<Self> },
}
