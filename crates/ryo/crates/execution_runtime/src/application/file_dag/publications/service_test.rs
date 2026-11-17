//! A binding's rows through `united`: its members' rows in member order, each row once, the coverage their union answers and
//! where its rows come from, from one file only where every member's do.
use super::service::united;
use super::types::{BoundRows, MemberRows};
use ryo_execution_domain::domain::completion::types::{CoverageState, FileLacks, MissingRows};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::lineage::service::one_file;
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;
use std::sync::Arc;

/// The members a binding unites.
#[derive(Clone, Copy, Debug)]
enum Members {
    /// Two members of one file each, the second lacking file 1's rows and repeating the first's row.
    OneFileEach,
    /// A member of one file each and a member made across the files.
    OneMixed,
    /// Two members whose columns differ.
    Mismatched,
    /// No member.
    None,
}

type Input = Members;
/// The united rows, their coverage and their lineage, as text.
type Value = Cow<'static, str>;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("members_of_one_file_each_unite_into_rows_of_one_file_each"),
        word: Word::Succeeds,
        input: || Members::OneFileEach,
        expect: Expect::Value(Cow::Borrowed(
            "[File, Number] [File(FileId(0)), Number(Integer(7)), File(FileId(0)), Number(Integer(8))] Partial { files: [Some(FileId(1))], lacks: [] } OneFile([InFile, Elsewhere])",
        )),
    },
    Case {
        name: CaseName("a_member_made_across_the_files_makes_the_union_s_rows_mixed"),
        word: Word::Succeeds,
        input: || Members::OneMixed,
        expect: Expect::Value(Cow::Borrowed(
            "[File, Number] [File(FileId(0)), Number(Integer(7)), File(FileId(0)), Number(Integer(8))] Partial { files: [Some(FileId(1))], lacks: [] } Mixed",
        )),
    },
    Case {
        name: CaseName("a_binding_no_producer_feeds_is_empty_of_its_declared_columns"),
        word: Word::Succeeds,
        input: || Members::None,
        expect: Expect::Value(Cow::Borrowed("[File, Number] [] Complete Mixed")),
    },
    Case {
        name: CaseName("members_whose_columns_differ_are_refused"),
        word: Word::Refuses,
        input: || Members::Mismatched,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
];

/// A table of `kinds` holding `rows`.
fn table(kinds: &[CellKind], rows: &[&[Cell]]) -> Arc<Table> {
    let mut table = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    for row in rows {
        push_row(&mut table, row);
    }
    Arc::new(table)
}

fn subject(members: Input) -> Result<Value, Failure> {
    let kinds = [CellKind::File, CellKind::Number];
    let row = |file: u32, number: i64| [Cell::File(FileId(file)), Cell::Number(Integer(number))];
    let first = MemberRows {
        rows: table(&kinds, &[row(0, 7).as_slice()]),
        coverage: CoverageState::Complete,
        lineage: one_file(&kinds),
    };
    let second = |lineage: Lineage| MemberRows {
        rows: table(&kinds, &[row(0, 7).as_slice(), row(0, 8).as_slice()]),
        coverage: CoverageState::Partial {
            files: vec![Some(FileId(1))],
            lacks: vec![FileLacks {
                file: Some(FileId(1)),
                missing: MissingRows::Any,
            }],
        },
        lineage,
    };
    let united = match members {
        Members::OneFileEach => united(&[first, second(one_file(&kinds))], kinds.to_vec()),
        Members::OneMixed => united(&[first, second(Lineage::Mixed)], kinds.to_vec()),
        Members::Mismatched => united(
            &[
                first,
                MemberRows {
                    rows: table(&[CellKind::File], &[]),
                    coverage: CoverageState::Complete,
                    lineage: Lineage::Mixed,
                },
            ],
            kinds.to_vec(),
        ),
        Members::None => united(&[], kinds.to_vec()),
    }?;
    match united {
        BoundRows::Held {
            table,
            coverage,
            lineage,
        } => Ok(Cow::Owned(format!(
            "{:?} {:?} {coverage:?} {lineage:?}",
            table.kinds, table.cells
        ))),
        BoundRows::Withheld(token) => Ok(Cow::Owned(format!("withheld {token:?}"))),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
