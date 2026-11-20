//! The `binding_facts` service's cases. Track P1's placeholder of the file track P2 writes: every relation answers that it is not read yet.

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
        name: CaseName("binding_event_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "binding_event",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("binding_read_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "binding_read",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("binding_use_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "binding_use",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("binding_shadow_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "binding_shadow",
        expect: Expect::Value(Fill::Absent(AbsenceCause::NotYetRead)),
    },
    Case {
        name: CaseName("binding_resolution_is_not_read_yet"),
        word: Word::Succeeds,
        input: || "binding_resolution",
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
        "binding_event" => Ok(super::service::binding_event_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::BindingEvent>(),
        )),
        "binding_read" => Ok(super::service::binding_read_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::BindingRead>(),
        )),
        "binding_use" => Ok(super::service::binding_use_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::BindingUse>(),
        )),
        "binding_shadow" => Ok(super::service::binding_shadow_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::BindingShadow>(),
        )),
        "binding_resolution" => Ok(super::service::binding_resolution_rows(
            &mut Empty::default(),
            words,
            &mut rows::<languages::BindingResolution>(),
        )),
        _ => Err(()),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
