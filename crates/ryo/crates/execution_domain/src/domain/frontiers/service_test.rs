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

/// What a row asks of the frontier service.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The project relation an input of a plan is, if any.
    ProjectRelation(PlanInput),
    /// The table the words of rows of one column of names (each a hash and a place) decode into.
    Decode(&'static [(u64, u32)]),
    /// The rows the project wrote of `package_dependency`, one column of these names, their names made again.
    ProjectInput(&'static [&'static str]),
}

/// What the frontier service answers.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The project relation.
    Relation(Option<Relation>),
    /// The rows, cells and columns of a decoded table.
    Table {
        rows: usize,
        cells: usize,
        columns: usize,
    },
    /// The rows found, whether their names hash as the distinct texts written, in order, and whether the input found is the
    /// input asked for.
    Projected {
        rows: usize,
        names_are_the_texts: bool,
        same_input: bool,
    },
}

type Input = Ask;
type Value = Answer;
type Failure = std::convert::Infallible;

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

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::ProjectRelation(input) => Answer::Relation(project_relation(input)),
        Ask::Decode(names) => {
            let table = decode_input(&words_of(names), &mut Symbols::default());
            Answer::Table {
                rows: table.count.0,
                cells: table.cells.len(),
                columns: table.kinds.len(),
            }
        }
        Ask::ProjectInput(texts) => {
            let mut project = Symbols::default();
            let words: Vec<u64> = texts
                .iter()
                .map(|text| {
                    u64::from(intern_derived(&mut project, ByteSlice(&[]), NameText(text)).0)
                })
                .collect();
            let columns = [FactColumn(words)];
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
            let mut distinct: Vec<&str> = Vec::new();
            for text in texts {
                if !distinct.contains(text) {
                    distinct.push(text);
                }
            }
            let written: Vec<Option<TextHash>> = distinct
                .iter()
                .map(|text| Some(hash_of(ByteSlice(text.as_bytes()))))
                .collect();
            Answer::Projected {
                rows: found.table.count.0,
                names_are_the_texts: hashes == written,
                same_input: found.input == input,
            }
        }
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("only_a_relation_the_project_wrote_is_a_project_relation"),
        word: Word::Succeeds,
        input: || Ask::ProjectRelation(PlanInput::Relation(Relation::PackageDependency)),
        expect: Expect::Value(Answer::Relation(Some(Relation::PackageDependency))),
    },
    Case {
        name: CaseName("a_relation_the_files_wrote_is_no_project_relation"),
        word: Word::Succeeds,
        input: || Ask::ProjectRelation(PlanInput::Relation(Relation::Function)),
        expect: Expect::Value(Answer::Relation(None)),
    },
    Case {
        name: CaseName("a_step_is_no_project_relation"),
        word: Word::Succeeds,
        input: || Ask::ProjectRelation(PlanInput::Step(StepId(0))),
        expect: Expect::Value(Answer::Relation(None)),
    },
    Case {
        name: CaseName("the_words_the_workers_kept_are_decoded_into_one_table"),
        word: Word::Succeeds,
        input: || Ask::Decode(&[(1, 0), (1, 0), (2, 0)]),
        expect: Expect::Value(Answer::Table {
            rows: 3,
            cells: 3,
            columns: 1,
        }),
    },
    Case {
        name: CaseName("the_rows_the_project_wrote_have_their_names_made_again"),
        word: Word::Succeeds,
        input: || Ask::ProjectInput(&["alpha", "beta"]),
        expect: Expect::Value(Answer::Projected {
            rows: 2,
            names_are_the_texts: true,
            same_input: true,
        }),
    },
    Case {
        name: CaseName("a_fact_two_languages_wrote_of_one_manifest_is_one_row"),
        word: Word::Succeeds,
        input: || Ask::ProjectInput(&["alpha", "beta", "alpha"]),
        expect: Expect::Value(Answer::Projected {
            rows: 2,
            names_are_the_texts: true,
            same_input: true,
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_values::domain::rows::types::Table;
