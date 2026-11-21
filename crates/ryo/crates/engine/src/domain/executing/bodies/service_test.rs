//! The bodies' cases, each a rule over a made table written as steps and encoded by the straight encoding: an inline body
//! asked for a row, present and absent; an answer kept by key; an `Or` over inline bodies; a pattern in call form, asked
//! through the memo and run once per key; a `Fork`.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::controlling::service::or_of;
use crate::domain::code::types::{MemoSite, Op};
use crate::domain::encoding::service::encoded;
use crate::domain::executing::bodies::service::asked_pattern;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::executing::fixtures::{Kept, Scene, pack};
use crate::domain::executing::service::run as run_program;
use crate::domain::made_tables::types::PatternAsk;
use crate::domain::memos::service::rows;
use crate::domain::programs::fixtures::{framed, held, table};
use crate::domain::programs::frames::types::RunMode;
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{MadeTableId, Polarity};
use ryo_language::domain::facts::types::CellWord;
use std::borrow::Cow;

type Input = Scene;
/// The cells found; then 99, the steps, the answers kept, and the staged cells, yielded cells and cursors left.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_inline_body_with_a_row_lets_the_candidate_go_on"),
        word: Word::Succeeds,
        input: || Scene::Exists(Polarity::Present, Kept::No),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 99, 18, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("absent_keeps_the_candidates_whose_body_has_no_row"),
        word: Word::Succeeds,
        input: || Scene::Exists(Polarity::Absent, Kept::No),
        expect: Expect::Value(Cow::Borrowed(&[3, 99, 15, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("an_answer_kept_by_key_is_asked_once_per_key"),
        word: Word::Succeeds,
        input: || Scene::Exists(Polarity::Present, Kept::ByKey),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 99, 16, 2, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("an_or_gives_every_row_of_every_body"),
        word: Word::Succeeds,
        input: || Scene::OrInline,
        expect: Expect::Value(Cow::Borrowed(&[
            10, 100, 10, 100, 3, 100, 99, 31, 0, 0, 0, 0, 0,
        ])),
    },
    Case {
        name: CaseName("a_pattern_in_call_form_runs_once_per_key"),
        word: Word::Succeeds,
        input: || Scene::OrPattern,
        expect: Expect::Value(Cow::Borrowed(&[10, 10, 99, 15, 2, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_fork_runs_each_slot_from_the_same_registers"),
        word: Word::Succeeds,
        input: || Scene::Fork,
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 2, 3, 3, 99, 23, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_body_that_ends_at_its_first_row_leaves_no_staged_row"),
        word: Word::Succeeds,
        input: || Scene::ExistsOverOr,
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 3, 99, 38, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_pattern_in_table_form_gives_the_collected_columns_of_its_keys_rows"),
        word: Word::Succeeds,
        input: || Scene::OrTable,
        expect: Expect::Value(Cow::Borrowed(&[7, 9, 7, 9, 99, 15, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_pattern_in_table_form_gives_each_collected_column_to_its_own_output"),
        word: Word::Succeeds,
        input: || Scene::OrTableBoth,
        expect: Expect::Value(Cow::Borrowed(&[
            10, 7, 10, 9, 10, 7, 10, 9, 99, 19, 0, 0, 0, 0, 0,
        ])),
    },
    Case {
        name: CaseName("a_pattern_in_table_form_asked_for_a_row_is_an_index_read"),
        word: Word::Succeeds,
        input: || Scene::ExistsTable,
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 99, 13, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_pattern_asked_back_by_key_runs_once_for_the_key"),
        word: Word::Succeeds,
        input: || Scene::AskedByKey,
        expect: Expect::Value(Cow::Borrowed(&[10, 1, 99, 3, 1])),
    },
];

fn subject(scene: Input) -> Result<Value, Failure> {
    let code = encoded(&pack(scene)).map_err(|_| ())?;
    let entry = code
        .rules
        .first()
        .and_then(|rule| rule.branches.first())
        .map(|branch| branch.entry)
        .ok_or(())?;
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(9);
    table(&mut held, 0, 2, &[&[1, 10], &[2, 10], &[3, 3]]);
    table(&mut held, 1, 2, &[&[10, 7], &[4, 8], &[10, 9]]);
    if matches!(scene, Scene::AskedByKey) {
        // The rule's `Or` holds the use of the pattern: its body record is what a `Use` stage would hand back.
        let body = programs
            .code
            .words
            .iter()
            .find(|word| word.op == Op::Or)
            .map(|word| or_of(*word).records)
            .ok_or(())?;
        let (answers, _found) = framed(&programs, &mut held, &TheLoop, |frame| {
            let key = [CellWord(10)];
            let ask = || PatternAsk {
                asker: MadeTableId(0),
                body,
                memo: MemoSite(0),
                key: &key,
            };
            let first = asked_pattern(frame, ask());
            let again = asked_pattern(frame, ask());
            let mut cells: Vec<u64> = rows(&frame.state.memos, first)
                .iter()
                .map(|cell| cell.0)
                .collect();
            cells.push(u64::from(first == again));
            cells
        });
        let mut all = answers;
        all.extend([
            99,
            held.state.work.steps.0,
            u64::try_from(held.state.memos.entries.len()).unwrap_or(u64::MAX),
        ]);
        return Ok(Cow::Owned(all));
    }
    let (_rows, found) = framed(&programs, &mut held, &TheLoop, |frame| {
        run_program(frame, entry, RunMode::Rows)
    });
    let mut all: Vec<u64> = found.rows.into_iter().flatten().collect();
    all.extend([
        99,
        held.state.work.steps.0,
        u64::try_from(held.state.memos.entries.len()).unwrap_or(u64::MAX),
        u64::try_from(held.state.staged.len()).unwrap_or(u64::MAX),
        u64::try_from(held.state.yielded.len()).unwrap_or(u64::MAX),
        u64::try_from(held.state.frames.len()).unwrap_or(u64::MAX),
        held.state.work.unbuilt.0,
    ]);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
