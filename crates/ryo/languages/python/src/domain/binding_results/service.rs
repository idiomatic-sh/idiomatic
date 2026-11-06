//! Encode static semantic categories without embedding lint policy.
use super::types::{EventKind, ReadContext, ReadState, ShadowRelationship, UseCertainty};
use ryo::languages::{Integer, NameText};
use super::constants;
/// Event category.
pub(crate) const fn event(kind: EventKind) -> NameText<'static> {
    match kind {
        EventKind::Wildcard => NameText(constants::WILDCARD),
        EventKind::Read => NameText(constants::READ),
        EventKind::Bind => NameText(constants::BIND),
        EventKind::Annotation => NameText(crate::domain::syntax::constants::ANNOTATION_SEED),
        EventKind::Delete => NameText(constants::DELETE),
        EventKind::Cleanup => NameText(constants::CLEANUP),
        EventKind::Global => NameText(constants::GLOBAL),
        EventKind::Nonlocal => NameText(constants::NONLOCAL),
    }
}
/// Explicit association state, distinct from lexical ownership and runtime initialization.
pub(crate) const fn state(value: ReadState) -> NameText<'static> {
    match value {
        ReadState::UniqueDeclaration => NameText(constants::UNIQUE_DECLARATION),
        ReadState::AlternativeDeclarations => NameText(constants::ALTERNATIVE_DECLARATIONS),
        ReadState::PossiblyUnbound => NameText(constants::POSSIBLY_UNBOUND),
        ReadState::Unbound => NameText(constants::UNBOUND),
        ReadState::Builtin { .. } => NameText(constants::BUILTIN),
        ReadState::DeclarationOrBuiltin { .. } => NameText(constants::DECLARATION_OR_BUILTIN),
        ReadState::Uncertain => NameText(constants::UNCERTAIN),
    }
}
/// Context of a reference, not a linter usage decision.
pub(crate) const fn context(value: ReadContext) -> NameText<'static> {
    match value {
        ReadContext::Value => NameText(constants::VALUE),
        ReadContext::Type => NameText(constants::TYPE),
        ReadContext::Deletion => NameText(constants::DELETION),
    }
}
/// Whether a real edge is the exclusive established candidate.
pub(crate) const fn certainty(value: UseCertainty) -> NameText<'static> {
    match value {
        UseCertainty::Unique => NameText(constants::UNIQUE),
        UseCertainty::Alternative => NameText(constants::ALTERNATIVE),
    }
}
/// Same-namespace replacement and outer lexical shadow remain distinct.
pub(crate) const fn relationship(value: ShadowRelationship) -> NameText<'static> {
    match value {
        ShadowRelationship::Replacement => NameText(constants::REPLACEMENT),
        ShadowRelationship::Lexical => NameText(constants::LEXICAL),
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
