//! The `answers` service: free functions named by verb that make the answer of a language for a relation, and say why it gives none, with no IO and no ports.

use super::types::{Answer, Expansions, ReadItem, Reader, Support};
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

/// The native rows of the semantic projection, and the trees a language added for reading under the element that holds them (a macro
/// call's arguments): the same rows as [`native_semantic_syntax`], with the added elements included.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::answers::service::expanded_semantic_syntax;
///
/// // A language that adds trees under a macro call answers its `semantic_syntax` slot with this reader.
/// let _support = expanded_semantic_syntax::<()>();
/// ```
#[must_use]
pub fn expanded_semantic_syntax<M>() -> Support<
    Reader<
        ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntax,
        M,
    >,
>
where
    M: 'static,
{
    Support::from(Answer::Reads(Reader {
        expansions: Expansions::Included,
        ..<super::types::NativeSemanticSyntax<M> as ReadItem>::READER
    }))
}

/// The edges of [`expanded_semantic_syntax`]: native parent, ordinal and field identities, and each added tree under its holder.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::answers::service::expanded_semantic_child;
///
/// // A language that adds trees under a macro call answers its `semantic_child` slot with this reader.
/// let _support = expanded_semantic_child::<()>();
/// ```
#[must_use]
pub fn expanded_semantic_child<M>() -> Support<
    Reader<
        ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChild,
        M,
    >,
>
where
    M: 'static,
{
    Support::from(Answer::Reads(Reader {
        expansions: Expansions::Included,
        ..<super::types::NativeSemanticChild<M> as ReadItem>::READER
    }))
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
