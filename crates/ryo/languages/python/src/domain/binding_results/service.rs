//! Encode static semantic categories without embedding lint policy.
use super::types::{EventKind, ReadContext, ReadState, ShadowRelationship, UseCertainty};
use ryo::languages::Integer;
/// Event category.
pub(crate) const fn event(kind: EventKind) -> &'static str {
    match kind {
        EventKind::Read => "read",
        EventKind::Bind => "bind",
        EventKind::Annotation => "annotation",
        EventKind::Delete => "delete",
        EventKind::Cleanup => "cleanup",
        EventKind::Global => "global",
        EventKind::Nonlocal => "nonlocal",
    }
}
/// Explicit association state, distinct from lexical ownership and runtime initialization.
pub(crate) const fn state(value: ReadState) -> &'static str {
    match value {
        ReadState::UniqueDeclaration => "unique-declaration",
        ReadState::AlternativeDeclarations => "alternative-declarations",
        ReadState::PossiblyUnbound => "possibly-unbound",
        ReadState::Unbound => "unbound",
        ReadState::Builtin { .. } => "builtin",
        ReadState::DeclarationOrBuiltin { .. } => "declaration-or-builtin",
        ReadState::Uncertain => "uncertain",
    }
}
/// Context of a reference, not a linter usage decision.
pub(crate) const fn context(value: ReadContext) -> &'static str {
    match value {
        ReadContext::Value => "value",
        ReadContext::Type => "type",
        ReadContext::Deletion => "deletion",
    }
}
/// Whether a real edge is the exclusive established candidate.
pub(crate) const fn certainty(value: UseCertainty) -> &'static str {
    match value {
        UseCertainty::Unique => "unique",
        UseCertainty::Alternative => "alternative",
    }
}
/// Same-namespace replacement and outer lexical shadow remain distinct.
pub(crate) const fn relationship(value: ShadowRelationship) -> &'static str {
    match value {
        ShadowRelationship::Replacement => "replacement",
        ShadowRelationship::Lexical => "lexical",
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
