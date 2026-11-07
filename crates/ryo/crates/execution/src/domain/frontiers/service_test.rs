//! The `frontiers` service's cases, on values alone.

use super::service::{decode_input, project_input, project_relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::service::push_row;

use ryo_language::domain::symbols::service::{hash_of, intern_derived};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Name, PlaceNumber, Relation, TextHash, Value as Cell, Words,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{Input as PlanInput, StepId};
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::WordSlice;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A relation the project wrote, one the files wrote and a step.
    ProjectRelations,
    /// The words three rows were kept as.
    DecodeInput,
    /// The rows of a relation the project wrote, their names made again.
    ProjectInput,
    /// The rows two languages wrote of one manifest, the same fact twice.
    ProjectInputTwice,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Up to three numbers a row answers; a place a row has no number for holds zero.
type Value = [usize; 3];
type Failure = ();

fn name(hash: u64, place: u32) -> Cell {
    Cell::Name(Name {
        hash: TextHash(hash),
        place: PlaceNumber(place),
    })
}

/// The words of a table of one column of names, one row for each of the given places and hashes.
fn words_of(names: &[(u64, u32)]) -> KeptWords {
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    for (hash, place) in names {
        push_row(&mut table, &[name(*hash, *place)]);
    }
    let mut words = Words::default();
    encode_table(&table, &[], &mut words);
    KeptWords(pack(WordSlice(&words.0)))
}

fn shape(table: &Table) -> Value {
    [table.count.0, table.cells.len(), table.kinds.len()]
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::ProjectRelations => [
            usize::from(
                project_relation(PlanInput::Relation(Relation::PackageDependency))
                    == Some(Relation::PackageDependency),
            ),
            usize::from(project_relation(PlanInput::Relation(Relation::Function)).is_none()),
            usize::from(project_relation(PlanInput::Step(StepId(0))).is_none()),
        ],
        Scenario::DecodeInput => {
            let rows = words_of(&[(1, 0), (1, 0), (2, 0)]);
            shape(&decode_input(&rows, &mut Symbols::default()))
        }
        Scenario::ProjectInputTwice => {
            let mut project = Symbols::default();
            let mut intern = |text| intern_derived(&mut project, ByteSlice(&[]), NameText(text));
            let (alpha, beta, again) = (intern("alpha"), intern("beta"), intern("alpha"));
            let words = [alpha, beta, again].map(|each| u64::from(each.0));
            let columns = [FactColumn(words.to_vec())];
            let view = ColumnsView {
                kinds: &[CellKind::Symbol],
                columns: &columns,
            };
            let input = PlanInput::Relation(Relation::PackageDependency);
            let found = project_input(view, input, &project, &mut Symbols::default());
            [found.table.count.0, 0, 0]
        }
        Scenario::ProjectInput => {
            let mut project = Symbols::default();
            let mut intern = |text| intern_derived(&mut project, ByteSlice(&[]), NameText(text));
            let (alpha, beta) = (intern("alpha"), intern("beta"));
            let columns = [FactColumn(vec![u64::from(alpha.0), u64::from(beta.0)])];
            let view = ColumnsView {
                kinds: &[CellKind::Symbol],
                columns: &columns,
            };
            let input = PlanInput::Relation(Relation::PackageDependency);
            let found = project_input(view, input, &project, &mut Symbols::default());
            let hashes: Vec<Option<TextHash>> = found
                .table
                .cells
                .iter()
                .map(|cell| match cell {
                    Cell::Name(each) => Some(each.hash),
                    Cell::Node(_)
                    | Cell::Symbol(_)
                    | Cell::File(_)
                    | Cell::Span(_)
                    | Cell::Number(_)
                    | Cell::Flag(_) => None,
                })
                .collect();
            let expected = [
                Some(hash_of(ByteSlice(b"alpha"))),
                Some(hash_of(ByteSlice(b"beta"))),
            ];
            [
                found.table.count.0,
                usize::from(hashes == expected),
                usize::from(found.input == input),
            ]
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("only_a_relation_the_project_wrote_is_a_project_relation"),
        word: Word::Succeeds,
        input: || Scenario::ProjectRelations,
        expect: Expect::Value([1, 1, 1]),
    },
    Case {
        name: CaseName("the_words_the_workers_kept_are_decoded_into_one_table"),
        word: Word::Succeeds,
        input: || Scenario::DecodeInput,
        expect: Expect::Value([3, 3, 1]),
    },
    Case {
        name: CaseName("the_rows_the_project_wrote_have_their_names_made_again"),
        word: Word::Succeeds,
        input: || Scenario::ProjectInput,
        expect: Expect::Value([2, 1, 1]),
    },
    Case {
        name: CaseName("a_fact_two_languages_wrote_of_one_manifest_is_one_row"),
        word: Word::Succeeds,
        input: || Scenario::ProjectInputTwice,
        expect: Expect::Value([2, 0, 0]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

use ryo_values::domain::rows::types::Table;
