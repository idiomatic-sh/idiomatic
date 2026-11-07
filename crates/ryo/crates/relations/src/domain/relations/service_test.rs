//! The `relations` service's cases, on values alone.

use crate::domain::terms::types::Symbol;

use super::service::{
    all, column_named, columns, kind, name, provenance_of, relation_named, relation_of,
};
use crate::domain::relations::types::{CellKind, NodeId, Offset, Provenance, Relation, Span, Value as Cell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::collections::relations::defines::item::DefinesColumn;
use crate::domain::relations::collections::relations::function::item::FunctionColumn;
use crate::domain::relations::service::from_bits;
use crate::domain::relations::service::is_key;
use crate::domain::relations::service::to_bits;
use crate::domain::relations::types::Column;
use crate::domain::labels::types::NameRef;
use crate::domain::relations::types::{ColumnKeyness, Flag, Integer};
use crate::domain::terms::types::{Vocabulary, WordBlock};

fn a_key_column_says_which_row_it_is() -> usize {
    let key = |relation: Relation, name_of: &str| {
        column_named(relation, NameRef(name_of))
            .is_some_and(|column| is_key(column) == ColumnKeyness::Key)
    };
    usize::from(key(Relation::SyntaxTree, "id"))
        + usize::from(key(Relation::SyntaxChild, "child"))
        + usize::from(key(Relation::Function, "id"))
        + usize::from(!key(Relation::Function, "name"))
        + usize::from(!key(Relation::SyntaxChild, "parent"))
}

fn the_engine_gives_the_files_and_the_manifests_are_the_projects() -> usize {
    usize::from(
        provenance_of(Relation::FilesystemFile) == Provenance::Engine
            && provenance_of(Relation::Function) == Provenance::Trees
            && provenance_of(Relation::PackageDependency) == Provenance::Manifests,
    )
}

/// A scenario: what a row asks of the catalog.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

fn every_relation_is_named_and_found_by_its_name() -> usize {
    all()
        .iter()
        .filter(|relation| relation_named(name(**relation)) == Some(**relation))
        .count()
}

fn a_name_no_relation_has_finds_nothing() -> usize {
    usize::from(relation_named(NameRef("nothing")).is_some())
}

fn function_has_four_columns() -> usize {
    columns(Relation::Function).len()
}

fn a_column_is_found_by_name_in_its_relation() -> usize {
    column_named(Relation::Function, NameRef("name")).map_or(0, |column| {
        usize::from(relation_of(column) == Relation::Function && kind(column) == CellKind::Symbol)
    })
}

fn values_survive_their_bits() -> usize {
    let values = [
        Cell::Node(NodeId(7)),
        Cell::Symbol(Symbol(3)),
        Cell::Span(Span {
            start: Offset(4),
            end: Offset(9),
        }),
        Cell::Number(Integer(-5)),
        Cell::Flag(Flag::Raised),
    ];
    let kinds = [
        CellKind::Node,
        CellKind::Symbol,
        CellKind::Span,
        CellKind::Number,
        CellKind::Flag,
    ];
    values
        .iter()
        .zip(kinds)
        .filter(|(value, kind)| from_bits(*kind, to_bits(**value)) == **value)
        .count()
}

/// Declaration kinds are closed; row identities remain open and keyness is per column.
fn a_column_has_its_vocabulary_and_its_keyness() -> usize {
    usize::from(
        super::service::vocabulary_of(Column::Defines(DefinesColumn::Kind))
            == Vocabulary::Closed(WordBlock::Declaration)
            && super::service::vocabulary_of(Column::Defines(DefinesColumn::Id))
                == Vocabulary::Open
            && is_key(Column::Function(FunctionColumn::Id)) == ColumnKeyness::Key
            && is_key(Column::Defines(DefinesColumn::Kind)) == ColumnKeyness::Plain,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_column_has_its_vocabulary_and_its_keyness"),
        word: Word::Succeeds,
        input: || a_column_has_its_vocabulary_and_its_keyness,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("optional capability identities round trip without weakening baseline"),
        word: Word::Succeeds,
        input: || optional_promises_are_closed,
        expect: Expect::Value(31),
    },
    Case {
        name: CaseName("every_relation_is_found_by_its_name"),
        word: Word::Succeeds,
        input: || every_relation_is_named_and_found_by_its_name,
        expect: Expect::Value(69),
    },
    Case {
        name: CaseName("a_name_no_relation_has_finds_nothing"),
        word: Word::Succeeds,
        input: || a_name_no_relation_has_finds_nothing,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("function_has_four_columns"),
        word: Word::Succeeds,
        input: || function_has_four_columns,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("a_column_is_found_by_name_in_its_relation"),
        word: Word::Succeeds,
        input: || a_column_is_found_by_name_in_its_relation,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_key_column_says_which_row_it_is"),
        word: Word::Succeeds,
        input: || a_key_column_says_which_row_it_is,
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("the_engine_gives_the_files_and_the_manifests_are_the_projects"),
        word: Word::Succeeds,
        input: || the_engine_gives_the_files_and_the_manifests_are_the_projects,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("values_survive_their_bits"),
        word: Word::Succeeds,
        input: || values_survive_their_bits,
        expect: Expect::Value(5),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Every optional promise names exactly one relation; required projections stay outside this set.
fn optional_promises_are_closed() -> usize {
    use super::service::{capability_relation, optional_capability};
    use crate::domain::capabilities::types::OptionalCapability;
    let capabilities = [
        OptionalCapability::ModuleIdentity,
        OptionalCapability::Identifier,
        OptionalCapability::QualifiedName,
        OptionalCapability::EmbeddedExpression,
        OptionalCapability::ExpressionShape,
        OptionalCapability::BindingEvent,
        OptionalCapability::BindingRead,
        OptionalCapability::BindingUse,
        OptionalCapability::BindingShadow,
        OptionalCapability::ImplicitSymbol,
        OptionalCapability::LiteralProperties,
        OptionalCapability::NumericLiteral,
        OptionalCapability::InterpolatedStringLiteral,
        OptionalCapability::BytesLiteral,
        OptionalCapability::FormatSpecification,
        OptionalCapability::TokenLink,
        OptionalCapability::LanguageTarget,
        OptionalCapability::StandardLibraryImport,
        OptionalCapability::CompilerFeature,
        OptionalCapability::ConstantKey,
        OptionalCapability::FormatParse,
        OptionalCapability::FormatField,
        OptionalCapability::BraceFormatParse,
        OptionalCapability::BraceFormatField,
        OptionalCapability::FunctionTerminal,
        OptionalCapability::ExpressionType,
        OptionalCapability::RawFormatSegment,
        OptionalCapability::RawFormatField,
        OptionalCapability::LiteralExpressionParse,
        OptionalCapability::ProjectNamespaceMembership,
    ];
    capabilities
        .into_iter()
        .filter(|capability| {
            optional_capability(capability_relation(*capability)) == Some(*capability)
        })
        .count()
        + usize::from(
            all()
                .iter()
                .filter(|relation| optional_capability(**relation).is_some())
                .count()
                == 30
                && optional_capability(Relation::SemanticSyntax).is_none()
                && optional_capability(Relation::SemanticChild).is_none(),
        )
}
