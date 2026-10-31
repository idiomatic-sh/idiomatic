//! The `answers` service: free functions named by verb that make the answer of a language for a relation, and say why it gives none, with no IO and no ports.

use super::types::{AbsenceCause, Answer, ReadItem, Reader, Support};
use crate::domain::relations::collections::relations::call_argument::item::CallArgumentColumn;
use crate::domain::relations::collections::relations::comment::item::CommentColumn;
use crate::domain::relations::collections::relations::defines::item::DefinesColumn;
use crate::domain::relations::collections::relations::entry::item::EntryColumn;
use crate::domain::relations::collections::relations::function::item::FunctionColumn;
use crate::domain::relations::collections::relations::mapping::item::MappingColumn;
use crate::domain::relations::collections::relations::mention::item::MentionColumn;
use crate::domain::relations::collections::relations::package_dependency::item::PackageDependencyColumn;
use crate::domain::relations::collections::relations::parameter::item::ParameterColumn;
use crate::domain::relations::collections::relations::prose::item::ProseColumn;
use crate::domain::relations::collections::relations::reference::item::ReferenceColumn;
use crate::domain::relations::collections::relations::resolves::item::ResolvesColumn;
use crate::domain::relations::collections::relations::sequence_element::item::SequenceElementColumn;
use crate::domain::relations::collections::relations::source_location::item::SourceLocationColumn;
use crate::domain::relations::collections::relations::static_call::item::StaticCallColumn;
use crate::domain::relations::collections::relations::string_literal::item::StringLiteralColumn;
use crate::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use crate::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use crate::domain::relations::types::{Column, ColumnKeyness};
use crate::domain::terms::types::{Vocabulary, WordBlock};

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
pub fn source_locations<M: 'static>() -> Support<
    Reader<
        crate::domain::relations::collections::relations::source_location::item::SourceLocation,
        M,
    >,
> {
    reads::<crate::domain::relations::collections::relations::source_location::item::SourceLocationReader<M>>()
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
                    $crate::domain::relations::collections::relations::$module::item::$variant,
                    Self::FileModel,
                >,
            > {
                $crate::domain::answers::service::absent($crate::domain::answers::types::AbsenceCause::NoSuchConstruct)
            }
        )*
    };
}

/// The words a column holds: one of a closed set for the columns that hold a kind the catalog shares across languages, any name or
/// number for the rest.
#[must_use]
pub fn vocabulary_of(column: Column) -> Vocabulary {
    if matches!(column, Column::Defines(DefinesColumn::Kind)) {
        Vocabulary::Closed(WordBlock::Declaration)
    } else if matches!(column, Column::Parameter(ParameterColumn::Kind)) {
        Vocabulary::Closed(WordBlock::Parameter)
    } else if matches!(
        column,
        Column::Entry(EntryColumn::Kind) | Column::SequenceElement(SequenceElementColumn::Kind)
    ) {
        Vocabulary::Closed(WordBlock::Value)
    } else if matches!(column, Column::Mapping(MappingColumn::Kind)) {
        Vocabulary::Closed(WordBlock::Mapping)
    } else if matches!(
        column,
        Column::PackageDependency(PackageDependencyColumn::Kind)
    ) {
        Vocabulary::Closed(WordBlock::Dependency)
    } else if matches!(
        column,
        Column::PackageDependency(PackageDependencyColumn::Origin)
    ) {
        Vocabulary::Closed(WordBlock::Origin)
    } else {
        Vocabulary::Open
    }
}

/// Whether a column says which row a row is: no two rows of the relation hold the same value in it,
/// so a projection that keeps it has no repeated rows to drop.
#[must_use]
pub fn is_key(column: Column) -> ColumnKeyness {
    if matches!(
        column,
        Column::SyntaxTree(SyntaxTreeColumn::Id)
            | Column::SourceLocation(SourceLocationColumn::Id)
            | Column::SyntaxChild(SyntaxChildColumn::Child)
            | Column::Function(FunctionColumn::Id)
            | Column::Comment(CommentColumn::Id)
            | Column::Prose(ProseColumn::Id)
            | Column::Reference(ReferenceColumn::Occurrence)
            | Column::Mention(MentionColumn::Occurrence)
            | Column::StaticCall(StaticCallColumn::Occurrence)
            | Column::CallArgument(CallArgumentColumn::Expression)
            | Column::Resolves(ResolvesColumn::Occurrence)
            | Column::StringLiteral(StringLiteralColumn::Occurrence)
    ) {
        ColumnKeyness::Key
    } else {
        ColumnKeyness::Plain
    }
}
