//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl. What the
//! language works out of a file once, before its readers run.

use crate::domain::brace_format::types::BraceFormatIndex;
use crate::domain::constant_keys::types::{ConstantKeyIndex, RetainedKeyIndex};
use crate::domain::expression_types::types::ExpressionTypeIndex;
use crate::domain::format_syntax::types::FormatParseIndex;
use crate::domain::indentation::types::IndentationIndex;
use crate::domain::terminal_flow::types::TerminalIndex;
use crate::domain::{labels::types::Field, syntax::types::Kind};
use ryo::languages::{Position, PropertyId, Symbol};
use std::cell::RefCell;

/// Where a scope says a name it assigns is bound instead of in itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingRedirect {
    /// `global`: in the module.
    Global,
    /// `nonlocal`: in the nearest enclosing function.
    Nonlocal,
}

/// One name a scope declares `global` or `nonlocal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScopeDeclaration {
    /// The scope the declaration is in.
    pub scope: Position,
    /// The name, as the file's names hold it.
    pub name: Symbol,
    /// Where the scope binds it.
    pub redirect: BindingRedirect,
}

/// A name within one lexical scope, independent of its declaration position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BindingKey {
    /// The lexical scope owning the declaration.
    pub scope: Position,
    /// The interned name within that scope.
    pub name: Symbol,
}

/// One declaration ordered by lexical scope, name, and source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LexicalBinding {
    /// The lexical scope and interned name.
    pub key: BindingKey,
    /// The declaration position in source order.
    pub position: Position,
}

/// File-local scope redirects and indexed declarations, shared by relation readers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Declarations {
    /// Configured language target, attached to every recognized source file.
    pub language_target: (i64, i64),
    /// Lazy canonical expression classes, invalidated between source revisions.
    pub expression_shapes: RefCell<ExpressionShapeIndex>,
    /// Lazy normalized constant-expression keys, invalidated between source revisions.
    pub constant_keys: RefCell<ConstantKeyIndex>,
    /// Lazy decoded percent-format parses, invalidated between source revisions.
    pub format_parsing: RefCell<FormatParseIndex>,
    /// Lazy individual raw-token percent-format parses, independently demanded.
    pub raw_format: RefCell<crate::domain::raw_format::types::RawFormatIndex>,
    /// Independent lazy outcomes for original literal expression carriers.
    pub literal_expression_parsing:
        RefCell<crate::domain::literal_expression_parsing::types::LiteralParseIndex>,
    /// Lazy decoded brace-format parses, invalidated between source revisions.
    pub brace_format: RefCell<BraceFormatIndex>,
    /// Lazy basic expression types, invalidated between source revisions.
    pub expression_types: RefCell<ExpressionTypeIndex>,
    /// Lazy terminal function summaries, invalidated between source revisions.
    pub terminal_flow: RefCell<TerminalIndex>,
    /// Every declared name.
    pub declared: Vec<ScopeDeclaration>,
    /// Definitions indexed by lexical scope and name, in source order.
    pub bindings: Vec<LexicalBinding>,
    /// Lazy lexical indentation regions retain their storage between files.
    pub indentation: RefCell<IndentationIndex>,
    /// Lazy provenance uncertainty, invalidated with every parse.
    pub qualification: RefCell<QualificationIndex>,
    /// Enabled embedded-expression roots, computed only for semantic readers.
    pub semantic: RefCell<SemanticIndex>,
    /// Lazy static binding observations; no analysis occurs for queries that do not request them.
    pub binding_analysis: RefCell<BindingAnalysisIndex>,
}

/// A lexical event that can invalidate otherwise known import or builtin provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QualificationHazardKind {
    /// A wildcard import may bind any name.
    Wildcard,
    /// Deletion removes one binding without introducing a new declaration.
    Delete(Symbol),
}
/// An event scoped and ordered independently of any linter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QualificationHazard {
    /// The namespace modified by the event.
    pub scope: Position,
    /// The source position of the event.
    pub position: Position,
    /// The event's effect on lexical provenance.
    pub kind: QualificationHazardKind,
}
/// Whether retained provenance events belong to the current parsed model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum QualificationState {
    /// The current source has not requested provenance.
    #[default]
    Fresh,
    /// Events have been indexed for the current source.
    Ready,
}
/// Lazily indexed uncertainty events; capacity survives reparsing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct QualificationIndex {
    /// Source-ordered namespace events.
    pub hazards: Vec<QualificationHazard>,
    /// Cache validity for the current file.
    pub state: QualificationState,
}

/// Retained selection of annotation expressions after lexical context classification.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SemanticIndex {
    /// The known enabled roots, in parser append order.
    pub roots: Vec<Position>,
    /// Whether these roots belong to the current parse.
    pub state: QualificationState,
}

/// Every component is tagged; a child identity cannot be mistaken for literal text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ShapeWord {
    /// The parsed node kind.
    Kind(Kind),
    /// A semantic field and its child class.
    Child(Field, Position),
    /// A significant operator or keyword.
    Token(Kind, Symbol),
    /// Canonical lexical identifier.
    Identifier(Symbol),
    /// A decoded literal segment, merged across implicit concatenation.
    Literal(Symbol),
    /// A typed scalar property captured by the parser.
    Property(PropertyId, Symbol),
}
/// A hash chooses candidates only; full stored keys establish equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ShapeHash(pub u64);
/// Whether the retained index belongs to a complete current tree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ShapeState {
    #[default]
    /// No source has requested construction since the last invalidation.
    Fresh,
    /// The source was fully indexed.
    Complete,
    /// One or more syntax inputs could not be represented completely.
    Partial,
}
/// Storage is kept across source edits; no index is built until requested.
pub(crate) type ExpressionShapeIndex =
    RetainedKeyIndex<Position, ShapeWord, ShapeHash, ShapeState>;
/// A sparse environment frame; branches inherit their entry state without copying it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct BindingFrameId(pub usize);
/// A frame and normalized lexical namespace key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct BindingFrameKey {
    /// Sparse environment identity.
    pub frame: BindingFrameId,
    /// Namespace and canonical name.
    pub key: BindingKey,
}
/// Whether a statically explored path can lack a declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingAbsence {
    /// Every explored alternative contains a declaration.
    Excluded,
    /// At least one explored alternative lacks a declaration (including deletion).
    Possible,
}
/// Whether all applicable binding effects have a supported static interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingKnowledge {
    /// The candidate set and possible absence are established by supported language events.
    Known,
    /// An unsupported or namespace-wide effect may supply additional candidates.
    Uncertain,
}
/// An arena-backed set of real declarations, with absence distinct from uncertainty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BindingAlternatives {
    /// Real declaration occurrences in the retained candidate arena.
    pub candidates: std::ops::Range<usize>,
    /// Whether an alternative can have no declaration.
    pub absence: BindingAbsence,
    /// Whether the known alternatives exhaust this static analysis.
    pub knowledge: BindingKnowledge,
}
/// One evaluation environment and its sparse mutation keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BindingFrame {
    /// Inherited environment; completed outer phases are never mutated by this frame.
    pub parent: Option<BindingFrameId>,
    /// Real execution-boundary syntax identity.
    pub phase: Position,
    /// Mutation keys first introduced by this frame; capacity is retained across files.
    pub touched: Vec<BindingKey>,
    /// Namespaces with wildcard effects, without inventing specific names.
    pub hazards: Vec<Position>,
}
/// Capacity-retaining sparse environments. No entry exists until binding analysis is requested.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BindingEnvironments {
    /// Frames are recycled between source revisions.
    pub frames: Vec<BindingFrame>,
    /// Number of frames belonging to the current source revision.
    pub active: usize,
    /// Only changed namespace keys are stored in each frame.
    pub slots: std::collections::HashMap<BindingFrameKey, BindingAlternatives>,
    /// Candidate identities are real definitions, never unknown-node sentinels.
    pub candidates: Vec<Position>,
    /// Retained union scratch; not stored once per branch or read.
    pub union: Vec<Position>,
    /// Retained deduplicated mutation keys during joins.
    pub keys: Vec<BindingKey>,
}

/// One language operation or structured-control delimiter in a static evaluation schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingAction {
    /// Read an existing name.
    Read,
    /// Introduce a real declaration identity.
    Bind,
    /// Declare a type without initializing a value.
    Annotation,
    /// Explicitly delete a name.
    Delete,
    /// Remove an exception target at handler exit.
    Cleanup,
    /// Declare module namespace routing.
    Global,
    /// Declare enclosing function namespace routing.
    Nonlocal,
    /// Begin an immediate or deferred evaluation boundary.
    BeginPhase,
    /// Finish a phase before dependent phases inherit it.
    EndPhase,
    /// Fork alternatives from the current environment.
    BeginAlternatives,
    /// Finish one arm and restart from the same entry environment.
    NextAlternative,
    /// Join all completed arms with explicit possible absence.
    EndAlternatives,
}
/// A real syntax occurrence scheduled inside one execution boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingStep {
    /// The actual occurrence, also used for implicit handler cleanup.
    pub occurrence: Position,
    /// Real module, callable, generator or deferred annotation identity.
    pub phase: Position,
    /// Language action, not a diagnostic or runtime initialization claim.
    pub action: BindingAction,
}
/// Deferred regions have grammar-specific entry points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingPhaseKind {
    /// Callable body evaluated after its enclosing definition phase.
    Callable,
    /// Generator target/filter/result evaluated separately from its first iterable.
    Generator,
    /// Deferred native or embedded type expression.
    Type,
}
/// A deferred region waits for its enclosing execution phase to complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingPhase {
    /// Real syntax root and public phase identity.
    pub root: Position,
    /// The execution boundary whose completed environment it inherits.
    pub parent: Position,
    /// Which grammar fields execute on entry.
    pub kind: BindingPhaseKind,
}
/// Whether native annotations use deferred source-name evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BindingAnnotationTiming {
    /// Native annotation expressions execute in the enclosing phase.
    #[default]
    Immediate,
    /// Future annotations defer native annotation name lookup.
    Deferred,
}
/// Reusable event and deferred-region buffers; constructed only on demand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BindingSchedule {
    /// Native annotation timing; quoted expressions are always separate deferred regions.
    pub annotations: BindingAnnotationTiming,
    /// Static event order, including structured alternatives rather than last-arm wins.
    pub steps: Vec<BindingStep>,
    /// Deferred regions in dependency order.
    pub phases: Vec<BindingPhase>,
}

/// Whether the current source has a complete static event analysis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BindingAnalysisState {
    /// No consumer has requested this source's binding observations.
    #[default]
    Fresh,
    /// All requested observations were constructed from supported syntax.
    Complete,
    /// Frontend or unsupported effects prevent complete relation enumeration.
    Incomplete,
}
/// One nested alternative group, storing only frame identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingBranch {
    /// Entry environment receiving the joined result.
    pub target: BindingFrameId,
    /// Offset into retained completed-arm scratch.
    pub start: usize,
}
/// Capacity-retaining lazy binding analysis, shared by the four relation readers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BindingAnalysisIndex {
    /// Current source lifecycle.
    pub state: BindingAnalysisState,
    /// Evaluation schedule; deferred phases are distinct from physical source order.
    pub schedule: BindingSchedule,
    /// Sparse branch and phase environments.
    pub environments: BindingEnvironments,
    /// Completed or active phase environments, keyed by real syntax boundary.
    pub phases: std::collections::HashMap<Position, BindingFrameId>,
    /// Nested branch entry frames.
    pub branches: Vec<BindingBranch>,
    /// Completed alternatives, reused by nested joins.
    pub arms: Vec<BindingFrameId>,
    /// Scratch for real lookup candidates, never builtin placeholders.
    pub candidates: Vec<crate::domain::binding_results::types::Candidate>,
    /// Mutation identities indexed by namespace/name for conservative repeated and exceptional regions.
    pub mutations: Vec<BindingMutation>,
    /// Evaluation-ordered events.
    pub events: Vec<crate::domain::binding_results::types::BindingResultEvent>,
    /// Explicit state for every analyzed reference.
    pub reads: Vec<crate::domain::binding_results::types::Read>,
    /// Real declaration associations.
    pub uses: Vec<crate::domain::binding_results::types::Usage>,
    /// Replacement and lexical shadow edges.
    pub shadows: Vec<crate::domain::binding_results::types::Shadow>,
}

/// One real mutation in a static execution phase, indexed without cloning namespaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingMutation {
    /// Affected namespace and canonical name.
    pub key: BindingKey,
    /// Real mutation syntax identity.
    pub occurrence: Position,
    /// Deferred/immediate evaluation boundary.
    pub phase: Position,
    /// Store, deletion, or exception cleanup.
    pub action: BindingAction,
}

/// Index of the next deferred binding phase to drain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PhaseCursor(pub usize);
