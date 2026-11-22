//! The `fact_tables` contract's cases of the answer as a whole: a relation filled whole answers every row in written
//! order; a partial answer says so and keeps its rows; a refused or unasked relation says so and has no row; the next
//! file's request forgets the last file's answer.

use super::super::fixtures::{EXPRESSION, expression_types, length, one, read, row_of};
use super::super::reading::service::{every_row, rows_of};
use super::super::service::{
    answer_of, asked, begun, finished, partial, refused, request_of, row, table_of, writer,
};
use super::super::types::{FactCells, FactKey, FactRowList, FactSet};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::facts::types::{CellWord, FactAnswer, FillerSentence, PartialRows};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{Missing, Position, Relation};

/// What a row asks of the answer.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// `language-target`, filled whole.
    FilledWhole,
    /// A filler says some rows may be missing, twice.
    PartialSaysSo,
    /// A relation refused, and one the request did not ask.
    RefusedAndUnasked,
    /// The next file's request after an answer.
    NextFile,
}

type Input = Scenario;
/// Up to four numbers a row reads, the rest zero.
type Value = [u64; 4];
/// No row of this part refuses.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_filled_whole_answers_every_row_in_written_order"),
        word: Word::Succeeds,
        input: || Scenario::FilledWhole,
        expect: Expect::Value([3, 1, 0, 0]),
    },
    Case {
        name: CaseName("a_partial_answer_says_the_first_cause_and_keeps_its_rows"),
        word: Word::Succeeds,
        input: || Scenario::PartialSaysSo,
        expect: Expect::Value([1, 2, 0, 0]),
    },
    Case {
        name: CaseName("a_refused_or_unasked_relation_says_so_and_has_no_row"),
        word: Word::Succeeds,
        input: || Scenario::RefusedAndUnasked,
        expect: Expect::Value([1, 1, 0, 1]),
    },
    Case {
        name: CaseName("the_next_file_request_forgets_the_last_file_answer"),
        word: Word::Succeeds,
        input: || Scenario::NextFile,
        expect: Expect::Value([0, 1, 0, 0]),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::FilledWhole => {
            let rows = vec![
                row_of(Relation::LanguageTarget, 0, 30),
                row_of(Relation::LanguageTarget, 0, 10),
                row_of(Relation::LanguageTarget, 0, 20),
            ];
            let set = one(Relation::LanguageTarget, rows);
            let table = table_of(&set, Relation::LanguageTarget);
            let all = every_row(table);
            let major = columns(Relation::LanguageTarget)
                .get(1)
                .copied()
                .ok_or(())?;
            let majors = read(table, major, FactRowList::Run(all));
            [
                u64::from(all.end.0 - all.first.0),
                u64::from(majors == [CellWord(30), CellWord(10), CellWord(20)]),
                length(rows_of(table, major, FactKey::Word(CellWord(30)))),
                0,
            ]
        }
        Scenario::PartialSaysSo => {
            let first = PartialRows {
                cause: FillerSentence("an unread construct".into()),
                missing: Missing::Any,
            };
            let second = PartialRows {
                cause: FillerSentence("another".into()),
                missing: Missing::Any,
            };
            let mut set = FactSet::default();
            begun(&mut set, &request_of(&[Relation::LanguageTarget]));
            let mut writing = writer(&mut set, Relation::LanguageTarget);
            row(
                &mut writing,
                FactCells(&row_of(Relation::LanguageTarget, 0, 1)),
            );
            partial(&mut writing, first.clone());
            row(
                &mut writing,
                FactCells(&row_of(Relation::LanguageTarget, 0, 2)),
            );
            partial(&mut writing, second);
            finished(writing);
            let all = every_row(table_of(&set, Relation::LanguageTarget));
            [
                u64::from(answer_of(&set, Relation::LanguageTarget) == FactAnswer::Partial(first)),
                u64::from(all.end.0 - all.first.0),
                0,
                0,
            ]
        }
        Scenario::RefusedAndUnasked => {
            let mut set = one(Relation::ExpressionType, expression_types());
            refused(
                &mut set,
                Relation::ExpressionType,
                AbsenceCause::NoSuchConstruct,
            );
            let request = request_of(&[Relation::ExpressionType]);
            [
                u64::from(
                    answer_of(&set, Relation::ExpressionType)
                        == FactAnswer::Refused(AbsenceCause::NoSuchConstruct),
                ),
                u64::from(
                    answer_of(&set, Relation::Comment)
                        == FactAnswer::Refused(AbsenceCause::NotYetRead),
                ),
                u64::from(every_row(table_of(&set, Relation::ExpressionType)).end.0),
                u64::from(asked(&request).eq([Relation::ExpressionType])),
            ]
        }
        Scenario::NextFile => {
            let mut set = one(Relation::ExpressionType, expression_types());
            begun(&mut set, &request_of(&[Relation::StringLiteral]));
            let table = table_of(&set, Relation::ExpressionType);
            [
                u64::from(every_row(table).end.0),
                u64::from(
                    answer_of(&set, Relation::ExpressionType)
                        == FactAnswer::Refused(AbsenceCause::NotYetRead),
                ),
                length(rows_of(table, EXPRESSION, FactKey::Node(Position(5)))),
                0,
            ]
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
