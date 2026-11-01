//! The `relations` service's cases, on values alone.

use crate::domain::terms::types::Symbol;

use super::service::{
    all, column_named, columns, folder_of, kind, name, provenance_of, relation_named, relation_of,
};
use super::types::{CellKind, NodeId, Offset, Provenance, Relation, Span, Value as Cell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answers::service::is_key;
use crate::domain::relations::service::from_bits;
use crate::domain::relations::service::to_bits;
use crate::domain::relations::types::NameRef;
use crate::domain::relations::types::{ColumnKeyness, Flag, Integer};

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

fn the_catalog_lists_exactly_the_folders_of_its_items() -> usize {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/domain/relations/collections/relations");
    let mut folders: Vec<String> = std::fs::read_dir(directory)
        .expect("a folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    folders.sort();
    let mut listed: Vec<String> = all()
        .iter()
        .map(|relation| folder_of(*relation).0.to_owned())
        .collect();
    listed.sort();
    usize::from(folders == listed)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("optional capability identities round trip without weakening baseline"),
        word: Word::Succeeds,
        input: || optional_promises_are_closed,
        expect: Expect::Value(18),
    },
    Case {
        name: CaseName("every_relation_is_found_by_its_name"),
        word: Word::Succeeds,
        input: || every_relation_is_named_and_found_by_its_name,
        expect: Expect::Value(53),
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
        name: CaseName("the_catalog_lists_exactly_the_folders_of_its_items"),
        word: Word::Succeeds,
        input: || the_catalog_lists_exactly_the_folders_of_its_items,
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
    use super::types::OptionalCapability;
    let capabilities = [
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
        OptionalCapability::InterpolatedStringLiteral,
        OptionalCapability::LanguageTarget,
        OptionalCapability::ConstantKey,
        OptionalCapability::FormatParse,
        OptionalCapability::FormatField,
        OptionalCapability::BraceFormatParse,
        OptionalCapability::BraceFormatField,
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
                == 17
                && optional_capability(Relation::SemanticSyntax).is_none()
                && optional_capability(Relation::SemanticChild).is_none(),
        )
}
