//! Encode static semantic categories without embedding lint policy.
use super::types::{EventKind, ReadContext, ReadState, ShadowRelationship, UseCertainty};
use ryo::languages::Integer;
use super::constants;
/// Event category.
pub(crate) const fn event(kind: EventKind) -> &'static str {
    match kind {
        EventKind::Wildcard => constants::WILDCARD,
        EventKind::Read => constants::READ,
        EventKind::Bind => constants::BIND,
        EventKind::Annotation => crate::domain::syntax::constants::ANNOTATION_SEED,
        EventKind::Delete => constants::DELETE,
        EventKind::Cleanup => constants::CLEANUP,
        EventKind::Global => constants::GLOBAL,
        EventKind::Nonlocal => constants::NONLOCAL,
    }
}
/// Explicit association state, distinct from lexical ownership and runtime initialization.
pub(crate) const fn state(value: ReadState) -> &'static str {
    match value {
        ReadState::UniqueDeclaration => constants::UNIQUE_DECLARATION,
        ReadState::AlternativeDeclarations => constants::ALTERNATIVE_DECLARATIONS,
        ReadState::PossiblyUnbound => constants::POSSIBLY_UNBOUND,
        ReadState::Unbound => constants::UNBOUND,
        ReadState::Builtin { .. } => constants::BUILTIN,
        ReadState::DeclarationOrBuiltin { .. } => constants::DECLARATION_OR_BUILTIN,
        ReadState::Uncertain => constants::UNCERTAIN,
    }
}
/// Context of a reference, not a linter usage decision.
pub(crate) const fn context(value: ReadContext) -> &'static str {
    match value {
        ReadContext::Value => constants::VALUE,
        ReadContext::Type => constants::TYPE,
        ReadContext::Deletion => constants::DELETION,
    }
}
/// Whether a real edge is the exclusive established candidate.
pub(crate) const fn certainty(value: UseCertainty) -> &'static str {
    match value {
        UseCertainty::Unique => constants::UNIQUE,
        UseCertainty::Alternative => constants::ALTERNATIVE,
    }
}
/// Same-namespace replacement and outer lexical shadow remain distinct.
pub(crate) const fn relationship(value: ShadowRelationship) -> &'static str {
    match value {
        ShadowRelationship::Replacement => constants::REPLACEMENT,
        ShadowRelationship::Lexical => constants::LEXICAL,
    }
}
/// Numeric availability is applicable only to states containing a builtin alternative.
pub(crate) const fn availability(value: ReadState) -> (Integer, Integer) {
    match value {
        ReadState::Builtin {
            since_major,
            since_minor,
        }
        | ReadState::DeclarationOrBuiltin {
            since_major,
            since_minor,
        } => (since_major, since_minor),
        ReadState::UniqueDeclaration
        | ReadState::AlternativeDeclarations
        | ReadState::PossiblyUnbound
        | ReadState::Unbound
        | ReadState::Uncertain => (Integer(0), Integer(0)),
    }
}
