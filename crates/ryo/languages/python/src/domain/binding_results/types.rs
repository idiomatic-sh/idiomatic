//! Real source identities and explicit states; no placeholder declaration represents a builtin or absence.
use ryo::languages::{Integer, Position, Symbol};

/// Static association at a read, not an assertion of runtime initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadState {
    /// One real declaration and no absent alternative.
    UniqueDeclaration,
    /// Multiple real declarations, every alternative bound.
    AlternativeDeclarations,
    /// At least one real declaration and at least one absent alternative.
    PossiblyUnbound,
    /// Supported analysis found no supplying declaration or builtin.
    Unbound,
    /// Static builtin association; version policy remains with the consumer.
    Builtin {
        /// Major component from the shared builtin inventory.
        since_major: Integer,
        /// Minor component from the shared builtin inventory.
        since_minor: Integer,
    },
    /// A real declaration on some alternatives, builtin fallback on all remaining alternatives.
    DeclarationOrBuiltin {
        /// Major component from the shared builtin inventory.
        since_major: Integer,
        /// Minor component from the shared builtin inventory.
        since_minor: Integer,
    },
    /// Namespace effects prevent a complete association, distinct from known absence.
    Uncertain,
}
/// Why this occurrence participates in name lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadContext {
    /// An ordinary evaluated expression.
    Value,
    /// A deferred annotation or explicit type expression.
    Type,
    /// An explicit deletion inspects the binding it removes.
    Deletion,
}
/// Cardinality/absence qualifier on a real declaration edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UseCertainty {
    /// The only candidate, with no absent alternative or unsupported effects.
    Unique,
    /// One of the possible candidates; consult the read state for possible absence.
    Alternative,
}
/// The relation between two real declaration identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShadowRelationship {
    /// A new store replaces an earlier candidate in the same namespace.
    Replacement,
    /// A local declaration shadows a declaration in an enclosing lexical namespace.
    Lexical,
}
/// One event in language evaluation order; declaration occurrences reuse defines identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingResultEvent {
    /// Actual declaration, reference, or cleanup syntax node.
    pub occurrence: Position,
    /// Canonical identifier.
    pub name: Symbol,
    /// Namespace affected by a store, or reference namespace for a read.
    pub scope: Position,
    /// Evaluation boundary, independent of source order.
    pub phase: Position,
    /// File-local schedule ordinal, not a byte offset or runtime timestamp.
    pub ordinal: Integer,
    /// Event category, shared with the schedule's executable actions.
    pub kind: EventKind,
}
/// Semantic events, excluding the scheduler's branch/phase control markers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EventKind {
    /// A namespace-wide import supplies no single declaration name.
    Wildcard,
    /// Read a name.
    Read,
    /// Introduce a concrete declaration identity.
    Bind,
    /// Declare a type without storing a value.
    Annotation,
    /// Explicitly remove a name.
    Delete,
    /// Remove an exception target after its handler.
    Cleanup,
    /// Declare module namespace routing.
    Global,
    /// Declare enclosing function namespace routing.
    Nonlocal,
}
/// Explicit result for one read, including known absence even when no use edge exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Read {
    /// Original semantic occurrence.
    pub reference: Position,
    /// Canonical identifier.
    pub name: Symbol,
    /// Lexical namespace of the reference.
    pub scope: Position,
    /// Evaluation boundary.
    pub phase: Position,
    /// Value/type/deletion context.
    pub context: ReadContext,
    /// Static association and applicable builtin version.
    pub state: ReadState,
}
/// One real candidate edge; absence and builtins never fabricate this row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Usage {
    /// Original semantic occurrence.
    pub reference: Position,
    /// Real defines identity.
    pub binding: Position,
    /// Namespace of the supplying declaration.
    pub scope: Position,
    /// Evaluation boundary of the read.
    pub phase: Position,
    /// Value/type/deletion context.
    pub context: ReadContext,
    /// Whether this edge is exclusive.
    pub certainty: UseCertainty,
}
/// Replacement or lexical-shadow association, not a linter redefinition decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shadow {
    /// New defines identity.
    pub binding: Position,
    /// Earlier or lexically enclosing defines identity.
    pub previous: Position,
    /// Namespace of the new declaration.
    pub scope: Position,
    /// Namespace of the previous declaration.
    pub previous_scope: Position,
    /// Evaluation boundary where the new declaration appears.
    pub phase: Position,
    /// Same-namespace replacement versus enclosing lexical shadow.
    pub relationship: ShadowRelationship,
}

/// A concrete candidate and its owning namespace, retained in lookup scratch storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    /// Actual defines identity.
    pub binding: Position,
    /// Namespace supplying it.
    pub scope: Position,
}

/// The Python version from which a builtin binding exists; zero for states without a builtin alternative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Availability {
    /// The major version.
    pub major: Integer,
    /// The minor version.
    pub minor: Integer,
}
