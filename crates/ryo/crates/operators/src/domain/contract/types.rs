//! Borrowed semantic contracts. The engine owns every acquired resource and output.

/// The engine's token for an input it withheld after the files: opaque to operators, which raise it as `Withheld` where the
/// input is first read and never read it as empty rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WithheldInput(pub u32);

/// The source length used to validate a composed byte interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceByteLength(pub usize);

/// A refused batch of renderings as the engine recorded it, by its place among the open file's refusals: carried by the fault
/// of the node that rendered and of every node that reads it, so each reader answers the refusal of the node it reads.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::contract::types::RenderRefusal;
/// assert_eq!(RenderRefusal(0), RenderRefusal(0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderRefusal(pub u32);

/// One portable semantic fault vocabulary for every borrowed operator call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortableOperatorFault {
    /// An input the engine withheld: it answers the fault with its own reason, found by the token.
    Withheld(WithheldInput),
    /// The engine stopped candidate acquisition or downstream delivery.
    Interrupted,
    /// The declared input column is absent.
    NoSuchColumn {
        /// Missing column index.
        index: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    /// A present cell is not a span.
    ShapeMismatch,
    /// Two edges of one partition and source share an order value: an ordered depth-first visit needs them distinct.
    DuplicateOrder,
    /// A span-bound extraction received an invalid interval.
    InvalidSpanBounds,
    /// A composed span exceeds the source or reverses its interval.
    InvalidSpan,
    /// An occurrence cannot be represented inside the validated physical source.
    InvalidSourceMatchSpan,
    /// A required named regular-expression capture is absent.
    MissingCapture {
        /// The capture the pattern lacks.
        capture: ryo_values::domain::plan_parts::types::LiteralText,
    },
    /// The supplied semantic options are not a valid operation.
    WrongOperation,
    /// A requested table key is absent from a table.
    UnboundKey,
    /// A language provider cannot render the requested syntax.
    UnsupportedRender,
    /// A resource the operation needs is not bound to the invocation, or a bound value does not fit it.
    WrongBinding,
    /// A parameter the operation reads has no binding.
    Unbound,
    /// A row the operation reads was not produced.
    NotRun,
    /// A comparison's test is not one of its family's.
    WrongTest,
    /// The ordering of an ordered match refused its rows: the derived fault, mapped 1:1 by the engine.
    Derived(ryo_derived::domain::fault::types::DerivedFault),
    /// The engine refused a batch of renderings: the refusal it recorded, which every reader of the node raises in turn.
    RenderRefused {
        /// The engine's record of the refusal.
        refusal: RenderRefusal,
    },
    /// A pattern a comparison compiled is no regular expression.
    Pattern(ryo_matching::inbound::use_cases::matching::errors::MatchingError),
}

/// What the already evaluated predicate decided about the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredicateOutcome {
    /// The predicate holds: the row is kept.
    Holds,
    /// The predicate fails: the row is dropped.
    Fails,
}

/// What a language provider answered for one render request, without its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderKind {
    /// A text was rendered.
    Known,
    /// The node has no rendering here.
    NotApplicable,
    /// The provider cannot render this syntax.
    Unsupported,
}

/// Whether a resolved cell is text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFound {
    /// The cell is text; it was appended to the caller's buffer.
    Text,
    /// The cell holds no text.
    NotText,
}

/// A text buffer the engine keeps between invocations so it keeps its capacity: a resolved, split, replaced
/// or rendered text.
#[derive(Debug, Default)]
pub struct TextScratch(pub String);

/// Byte ranges into a scratch text, kept between invocations so they keep their capacity: the slots of a path
/// capture, or where each rendering of a batch is.
#[derive(Debug, Default)]
pub struct RangeScratch(pub Vec<std::ops::Range<usize>>);

/// The row buffers one evaluation fills, kept by the engine between invocations so they keep their capacity.
#[derive(Debug, Default)]
pub struct RowScratch {
    /// The row being answered.
    pub row: Vec<ryo_relations::domain::relations::types::Value>,
    /// A second row: a projection, a prefix, a population row.
    pub other: Vec<ryo_relations::domain::relations::types::Value>,
    /// Captured or derived values.
    pub captured: Vec<ryo_relations::domain::relations::types::Value>,
    /// A key.
    pub key: Vec<ryo_relations::domain::relations::types::Value>,
    /// A partition's key.
    pub partition: Vec<ryo_relations::domain::relations::types::Value>,
    /// A resolved or built text.
    pub text: TextScratch,
    /// The word being split.
    pub word: TextScratch,
    /// The text a replacement builds.
    pub replaced: TextScratch,
    /// The slot ranges of a path capture, or the places of a batch's renderings in `text`.
    pub slots: RangeScratch,
    /// Columns of a prepared structure.
    pub columns: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The node of each render request.
    pub nodes: Vec<ryo_relations::domain::relations::types::NodeId>,
    /// What the provider answered for each render request.
    pub rendered: Vec<RenderKind>,
}
