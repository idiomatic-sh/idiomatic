//! Borrowed semantic contracts. The engine owns every acquired resource and output.

/// The source length used to validate a composed byte interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceByteLength(pub usize);

/// One portable semantic fault vocabulary for every borrowed operator call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableOperatorFault {
    /// The engine stopped candidate acquisition or downstream delivery.
    Interrupted,
    /// The declared input column is absent.
    NoSuchColumn {
        /// Missing column index.
        index: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    /// A present cell is not a span.
    ShapeMismatch,
    /// A span-bound extraction received an invalid interval.
    InvalidSpanBounds,
    /// A composed span exceeds the source or reverses its interval.
    InvalidSpan,
    /// An occurrence cannot be represented inside the validated physical source.
    InvalidSourceMatchSpan,
    /// A required named regular-expression capture is absent.
    MissingCapture,
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
}

/// What the already evaluated predicate decided about the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredicateOutcome {
    /// The predicate holds: the row is kept.
    Holds,
    /// The predicate fails: the row is dropped.
    Fails,
}

/// Whether a row or value was seen before in this invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstOccurrence {
    /// This is the first time it is seen.
    First,
    /// It was seen before.
    Repeat,
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

/// Whether a population row was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowRead {
    /// The row was copied into the caller's buffer.
    Read,
    /// No row is at that place.
    Missing,
}

/// Whether the engine hands the right side of a join in logical columns or in physical tree columns, which
/// the declared view remaps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightColumns {
    /// The right side already answers the requested logical view.
    Logical,
    /// The right side answers physical tree columns; the view remaps them.
    Physical,
}

/// One group of a grouped population, as the engine prepared it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupSummary {
    /// The first row of the group, in arrival order.
    pub first: ryo_values::domain::rows::types::RowIndex,
    /// The rows the group counts.
    pub counted: ryo_relations::domain::relations::types::Count,
    /// The digest of the group's distinct values; zero when none was asked for.
    pub digest: ryo_relations::domain::relations::types::Integer,
}

/// A closure partition's sources, or the vertices one walk reached, by their place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VertexPlace(pub usize);

/// The occurrences of a prepared matcher in a text, found through the matching port the edge composed.
pub(crate) trait Occurrences {
    /// Visits each match of `regex` in `text`, stopping when `each` breaks.
    fn matches(
        &self,
        regex: &ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
        text: ryo_matching::inbound::use_cases::matching::types::SearchText<'_>,
        each: &mut dyn FnMut(
            ryo_matching::inbound::use_cases::matching::types::TextSpan,
        ) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()>;
    /// Visits each match of `regex` in `text` with the span of its named group, stopping when `each` breaks.
    ///
    /// # Errors
    ///
    /// `MissingCapture` when the matcher has no group of that name.
    fn captures(
        &self,
        regex: &ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
        text: ryo_matching::inbound::use_cases::matching::types::SearchText<'_>,
        group: ryo_matching::inbound::use_cases::matching::types::GroupName<'_>,
        each: &mut dyn FnMut(
            ryo_matching::inbound::use_cases::matching::types::TextSpan,
            ryo_matching::inbound::use_cases::matching::types::TextSpan,
        ) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, PortableOperatorFault>;
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

/// The engine's side of one evaluation that every kind reads: delivery, the bound predicate, the
/// invocation's distinct state, symbols, populations, groups and renderings.
pub trait RowDriver {
    /// Delivers one output row downstream.
    ///
    /// # Errors
    ///
    /// The engine retained a downstream error; every later call stays interrupted.
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), PortableOperatorFault>;
    /// Evaluates the operation's bound predicate program over a row.
    ///
    /// # Errors
    ///
    /// The engine retained an error evaluating the predicate.
    fn holds(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<PredicateOutcome, PortableOperatorFault>;
    /// Records a row in the invocation's first-occurrence state.
    ///
    /// # Errors
    ///
    /// The engine retained an error: no distinct state is bound, or its schema was refused.
    fn first_time(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<FirstOccurrence, PortableOperatorFault>;
    /// Counts rows the step found at one of its inputs.
    fn found(
        &mut self,
        input: ryo_values::domain::plan_parts::types::InputNumber,
        rows: ryo_relations::domain::relations::types::Count,
    );
    /// Resolves a cell to its text and appends it to `target`; a cell that is not text leaves it as it was.
    fn text(
        &mut self,
        value: ryo_relations::domain::relations::types::Value,
        target: &mut TextScratch,
    ) -> TextFound;
    /// Interns a text as a derived symbol of the file being read.
    fn mint(
        &mut self,
        text: ryo_values::domain::row_texts::types::CellText<'_>,
    ) -> ryo_relations::domain::relations::types::Value;
    /// How many rows a population holds.
    ///
    /// # Errors
    ///
    /// The engine retained an error: the population is absent.
    fn population(
        &mut self,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<ryo_values::domain::rows::types::RowCount, PortableOperatorFault>;
    /// Copies one row of a population into `row`.
    fn population_row(
        &mut self,
        input: ryo_values::domain::plan_parts::types::InputNumber,
        at: ryo_values::domain::rows::types::RowIndex,
        row: &mut Vec<ryo_relations::domain::relations::types::Value>,
    ) -> RowRead;
    /// How many groups the engine prepared for a grouped population.
    ///
    /// # Errors
    ///
    /// The engine retained an error: no grouping is bound.
    fn groups(
        &mut self,
    ) -> Result<ryo_relations::domain::relations::types::Count, PortableOperatorFault>;
    /// One prepared group.
    fn group(&self, at: VertexPlace) -> Option<GroupSummary>;
    /// Renders each node in the operation's syntax role and keeps the answers for `rendered`.
    ///
    /// # Errors
    ///
    /// The engine retained an error: no semantic session, or a batch of another shape.
    fn render(
        &mut self,
        nodes: &[ryo_relations::domain::relations::types::NodeId],
    ) -> Result<(), PortableOperatorFault>;
    /// Copies the kept answer of one rendered node into `text`.
    fn rendered(
        &self,
        at: ryo_values::domain::rows::types::RowIndex,
        text: &mut TextScratch,
    ) -> RenderKind;
    /// Whether the join's right side answers logical or physical columns.
    fn right_columns(&self) -> RightColumns;
}

impl From<bool> for FirstOccurrence {
    /// A row reported first-time is `First`.
    fn from(first: bool) -> Self {
        if first { Self::First } else { Self::Repeat }
    }
}
