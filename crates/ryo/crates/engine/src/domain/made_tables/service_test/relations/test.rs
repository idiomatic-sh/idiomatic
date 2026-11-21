//! The cases of a relation read through the seam, each on a pack the contract's encoding wrote: a key asked of the
//! language alone and once, a whole fill after keys, a key of two columns, a language that answers every row to a key, and
//! the faults of a relation the language refuses or writes in part.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::RelationNumber;
use crate::domain::filled::errors::FilledError;
use crate::domain::made_tables::fixtures::packs::constants;
use crate::domain::made_tables::fixtures::packs::{FIRST, SECOND, TOLD};
use crate::domain::made_tables::fixtures::reads::{asked, prober, told_of};
use crate::domain::made_tables::fixtures::{Telling, World, framed, site_of, world_with};
use crate::domain::made_tables::sources::service::{relation_rows, scanned};
use crate::domain::made_tables::types::{KeyFrom, ReadSource};
use crate::domain::steps::types::{ReadColumn, StepRows};
use ryo_language::domain::facts::types::FillerSentence;
use ryo_language::domain::facts::types::PartialRows;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Missing, Relation};
use std::borrow::Cow;

/// What a case reads through the seam.
#[derive(Clone, Copy, Debug)]
enum Read {
    /// A relation asked for a key from a row, twice, then for another key.
    KeyAlone,
    /// A relation asked for a key, then for every row, then for a constant key.
    WholeAfterKeys,
    /// A site of two columns asked for a key from a row.
    TwoColumns,
    /// A language that answers every row to a key.
    WholeToAKey,
    /// A relation the language refuses.
    Refused,
    /// A relation the language writes in part.
    Partial,
}

type Input = Read;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows.
const ROWS: [[u64; 2]; 5] = [[1, 10], [2, 20], [1, 30], [3, 10], [1, 40]];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_read_by_key_asks_the_language_for_that_key_alone"),
        word: Word::Succeeds,
        input: || Read::KeyAlone,
        expect: Expect::Value(Cow::Borrowed(
            "[0, 1, 2] filled 3 indexed 0 | [0, 1, 2] filled 0 indexed 0 | [3] filled 1 indexed 0 | asked: first 1, first 3",
        )),
    },
    Case {
        name: CaseName("a_whole_fill_after_keyed_fills_reads_each_row_once"),
        word: Word::Succeeds,
        input: || Read::WholeAfterKeys,
        expect: Expect::Value(Cow::Borrowed(
            "[0] filled 1 indexed 0 | rows 1..6 filled 5 | [1, 3, 5] filled 0 indexed 5 | [2] filled 0 indexed 0 | holds 6 rows | asked: first 2, whole",
        )),
    },
    Case {
        name: CaseName("a_key_of_two_columns_asks_one_and_keeps_the_rows_that_hold_the_other"),
        word: Word::Succeeds,
        input: || Read::TwoColumns,
        expect: Expect::Value(Cow::Borrowed(
            "[0] filled 1 indexed 0 | [] filled 0 indexed 0 | asked: second 30",
        )),
    },
    Case {
        name: CaseName("every_row_answered_to_a_key_makes_the_relation_whole"),
        word: Word::Succeeds,
        input: || Read::WholeToAKey,
        expect: Expect::Value(Cow::Borrowed(
            "[0, 2, 4] filled 5 indexed 5 | [3] filled 0 indexed 0 | asked: first 1",
        )),
    },
    Case {
        name: CaseName("a_relation_the_language_refuses_is_a_fault_naming_it"),
        word: Word::Refuses,
        input: || Read::Refused,
        expect: Expect::Error(FilledError::Refused {
            relation: Relation::TokenLink,
            cause: AbsenceCause::NotYetRead,
        }),
    },
    Case {
        name: CaseName("a_relation_written_in_part_is_a_fault_naming_it"),
        word: Word::Refuses,
        input: || Read::Partial,
        expect: Expect::Refusal(|fault| {
            if *fault
                == (FilledError::Partial {
                    relation: Relation::TokenLink,
                    rows: PartialRows {
                        cause: FillerSentence("an unread stretch".into()),
                        missing: Missing::Any,
                    },
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// The world of a case: table 0 holds constants, and the table after it names the site the case reads.
fn world_of(read: Read) -> Result<World, FilledError> {
    let told = StepRows::Filled(TOLD);
    let columns = match read {
        Read::TwoColumns => vec![ReadColumn::Filled(SECOND), ReadColumn::Filled(FIRST)],
        Read::KeyAlone
        | Read::WholeAfterKeys
        | Read::WholeToAKey
        | Read::Refused
        | Read::Partial => vec![ReadColumn::Filled(FIRST)],
    };
    let telling = match read {
        Read::WholeToAKey => Telling::WholeToAKey,
        Read::Refused => Telling::Refusing,
        Read::Partial => Telling::Partial,
        Read::KeyAlone | Read::WholeAfterKeys | Read::TwoColumns => Telling::Keyed,
    };
    world_with(
        vec![constants(&[1, 4, 3]), prober(told, columns)],
        Vec::new(),
        &ROWS,
        telling,
    )
}

fn subject(read: Input) -> Result<Value, Failure> {
    let mut world = world_of(read)?;
    let relation_site = site_of(&world.programs.code, 1, &[0]);
    let relation = ReadSource::Filled(RelationNumber(0));
    let said = match read {
        Read::KeyAlone => {
            let one = asked(&mut world, relation_site, &[1], KeyFrom::Row)?;
            let again = asked(&mut world, relation_site, &[1], KeyFrom::Row)?;
            let other = asked(&mut world, relation_site, &[3], KeyFrom::Row)?;
            format!("{one} | {again} | {other} | asked: {}", told_of(&world))
        }
        Read::WholeAfterKeys => {
            let one = asked(&mut world, relation_site, &[2], KeyFrom::Row)?;
            let whole = {
                let (mut frame, looped) = framed(&mut world);
                scanned(&mut frame, looped, relation)?
            };
            let constant = asked(&mut world, relation_site, &[1], KeyFrom::Constants)?;
            let row = asked(&mut world, relation_site, &[2], KeyFrom::Row)?;
            format!(
                "{one} | rows {}..{} filled {} | {constant} | {row} | holds {} rows | asked: {}",
                whole.rows.start.0,
                whole.rows.end.0,
                whole.filled.0,
                relation_rows(&world.store, RelationNumber(0)).0,
                told_of(&world)
            )
        }
        Read::TwoColumns => {
            let site = site_of(&world.programs.code, 1, &[1, 0]);
            let one = asked(&mut world, site, &[30, 1], KeyFrom::Row)?;
            let none = asked(&mut world, site, &[30, 2], KeyFrom::Row)?;
            format!("{one} | {none} | asked: {}", told_of(&world))
        }
        Read::WholeToAKey => {
            let one = asked(&mut world, relation_site, &[1], KeyFrom::Row)?;
            let other = asked(&mut world, relation_site, &[3], KeyFrom::Row)?;
            format!("{one} | {other} | asked: {}", told_of(&world))
        }
        Read::Refused | Read::Partial => {
            let (mut frame, looped) = framed(&mut world);
            let first = scanned(&mut frame, looped, relation).map(|rows| rows.filled.0);
            // A second read answers the same fault and asks the language nothing.
            let second = scanned(&mut frame, looped, relation).map(|rows| rows.filled.0);
            drop(frame);
            assert_eq!(first, second);
            assert_eq!(world.told.asked.len(), 1);
            format!("{}", first?)
        }
    };
    Ok(Cow::Owned(said))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
