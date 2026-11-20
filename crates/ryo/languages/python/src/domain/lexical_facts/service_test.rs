//! The `lexical_facts` service's cases. Track P1's placeholder of the file track P2 writes: every relation answers that it is not read yet.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::view::types::{ModelPart, ModelRead, ModelView, SemanticModel};
use ryo::languages::{
    self, AbsenceCause, ByteSlice, CatalogRelation, FileId, Fill, Missing, NodeTable, ReportText,
    Rows,
};

/// A view of a file with no node.
#[derive(Default)]
struct Empty {
    /// Its model.
    model: SemanticModel,
    /// Its table.
    table: NodeTable,
}

impl ModelView for Empty {
    fn semantics(&mut self, _part: ModelPart) -> ModelRead<'_> {
        ModelRead {
            model: &self.model,
            table: &self.table,
            source: ByteSlice(&[]),
            file: FileId(0),
        }
    }
}

/// Rows no filler writes.
struct Unwritten;

impl<Row> Rows<Row> for Unwritten {
    fn write(&mut self, _row: Row) {
        unreachable!("a placeholder writes no row")
    }

    fn partial(&mut self, _cause: ReportText<'_>, _missing: Missing) {}

    fn partial_at(&mut self, _file: FileId, _cause: ReportText<'_>, _missing: Missing) {}
}

/// A relation, by its name.
type Input = &'static str;
type Value = Fill;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("scope_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "scope",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("defines_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "defines",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("reference_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "reference",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("resolves_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "resolves",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("qualified_name_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "qualified_name",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("implicit_symbol_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "implicit_symbol",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
];

/// The rows of one relation, unwritten.
fn rows<R: CatalogRelation>() -> impl Rows<R::Row> {
    Unwritten
}

fn subject(relation: Input) -> Result<Value, Failure> {
    let words = &mut languages::facts::file_words(&languages::facts::base_words(&[]));
    match relation {
        "scope" => Ok(super::service::scope_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::Scope>(),
        )),
        "defines" => Ok(super::service::defines_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::Defines>(),
        )),
        "reference" => Ok(super::service::reference_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::Reference>(),
        )),
        "resolves" => Ok(super::service::resolves_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::Resolves>(),
        )),
        "qualified_name" => Ok(super::service::qualified_name_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::QualifiedName>(),
        )),
        "implicit_symbol" => Ok(super::service::implicit_symbol_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::ImplicitSymbol>(),
        )),
        _ => Err(()),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
