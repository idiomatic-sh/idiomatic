//! The `answers` service: free functions named by verb that make the answer of a language for a relation, and say why it gives none, with no IO and no ports.

use super::types::{Answer, ReadItem, Reader, Support};
use ryo_relations::domain::answers::types::AbsenceCause;

/// The answer of a language that does not read a relation, for this reason.
#[must_use]
pub fn absent<R>(reason: AbsenceCause) -> Support<R> {
    Support::from(Answer::NotInThisLanguage(reason))
}

/// The answer of a language that reads a relation through the item `I`: the only way to make one, so a reader written beside the
/// item is no answer.
#[must_use]
pub fn reads<I>() -> Support<Reader<I::Relation, I::Model>>
where
    I: ReadItem,
{
    Support::from(Answer::Reads(I::READER))
}

/// Offers the engine's indexed coordinates while each provider explicitly declares support.
#[must_use]
pub fn source_locations<M>() -> Support<
    Reader<
        ryo_relations::domain::relations::collections::relations::source_location::item::SourceLocation,
        M,
    >,
>
where
    M: 'static,
{
    reads::<super::types::SourceLocationReader<M>>()
}

/// For `impl ParseUseCase`: answers every syntax relation as not in the language. It is for the test doubles of this crate
/// only, and is not exported: a language answers each relation by hand, so that the compiler lists what it has not.
macro_rules! absent_syntax {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest_variant:ident $manifest_module:ident $manifest_row:ident $manifest_column:ident ; )* }
        engine { $( $engine_variant:ident $engine_module:ident $engine_row:ident $engine_column:ident ; )* }
    ) => {
        $(
            fn $module(
                &self,
            ) -> $crate::domain::answers::types::Support<
                $crate::domain::answers::types::Reader<
                    ryo_relations::domain::relations::collections::relations::$module::item::$variant,
                    Self::FileModel,
                >,
            > {
                $crate::domain::answers::service::absent(ryo_relations::domain::answers::types::AbsenceCause::NoSuchConstruct)
            }
        )*
    };
}

/// Preserve native syntax exactly in the explicit semantic projection.
#[must_use]
pub fn native_semantic_syntax<M>() -> Support<
    Reader<
        ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntax,
        M,
    >,
>
where
    M: 'static,
{
    reads::<super::types::NativeSemanticSyntax<M>>()
}

/// Preserve native parent, ordinal, and field identities in the semantic projection.
#[must_use]
pub fn native_semantic_child<M>() -> Support<
    Reader<
        ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChild,
        M,
    >,
>
where
    M: 'static,
{
    reads::<super::types::NativeSemanticChild<M>>()
}
