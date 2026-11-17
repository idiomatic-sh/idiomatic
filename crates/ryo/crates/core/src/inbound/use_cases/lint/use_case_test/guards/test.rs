//! The guards on what a run reads, hands and keeps, each with the breach planted that it must find.
use super::fixtures::{Between, Breach, Change, Damage, Handed, Held, changed, damaged, handover, warm};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which guard a row holds, over what.
#[derive(Clone, Copy, Debug)]
enum Guard {
    Warm(Between),
    Handover(Handed),
    Damaged(Damage),
    Changed(Change, Held),
}

type Input = Guard;
/// The files the guarded run read and the rows it answered; for a handover, the files that handed rows and the most rows one handed.
type Value = (usize, u64);
type Failure = Breach;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_unchanged_warm_run_over_the_store_on_disk_reads_no_file"),
        word: Word::Succeeds,
        input: || Guard::Warm(Between::Kept),
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("a_warm_run_over_a_store_that_kept_nothing_reads_every_file"),
        word: Word::Refuses,
        input: || Guard::Warm(Between::Forgotten),
        expect: Expect::Error(Breach::Read(3)),
    },
    Case {
        name: CaseName("no_file_hands_a_rule_more_than_four_rows_for_each_row_of_its_tree"),
        word: Word::Succeeds,
        input: || Guard::Handover(Handed::Copies),
        expect: Expect::Value((3, 1)),
    },
    Case {
        name: CaseName("a_rule_that_pairs_the_rows_of_a_file_is_handed_more_than_four_rows_for_each"),
        word: Word::Refuses,
        input: || Guard::Handover(Handed::Pairs),
        expect: Expect::Error(Breach::Wide {
            handed: 25,
            parsed: 5,
        }),
    },
    Case {
        name: CaseName("a_store_cut_short_answers_as_if_there_were_none"),
        word: Word::Succeeds,
        input: || Guard::Damaged(Damage::Cut),
        expect: Expect::Value((3, 1)),
    },
    Case {
        name: CaseName("a_store_replaced_by_other_bytes_answers_as_if_there_were_none"),
        word: Word::Succeeds,
        input: || Guard::Damaged(Damage::Replaced),
        expect: Expect::Value((3, 1)),
    },
    Case {
        name: CaseName("a_store_with_a_byte_flipped_answers_as_if_there_were_none"),
        word: Word::Succeeds,
        input: || Guard::Damaged(Damage::Flipped),
        expect: Expect::Value((3, 1)),
    },
    Case {
        name: CaseName("a_store_removed_answers_as_if_there_were_none"),
        word: Word::Succeeds,
        input: || Guard::Damaged(Damage::Removed),
        expect: Expect::Value((3, 1)),
    },
    Case {
        name: CaseName("a_store_left_as_it_was_is_used"),
        word: Word::Refuses,
        input: || Guard::Damaged(Damage::Untouched),
        expect: Expect::Error(Breach::Kept(0)),
    },
    Case {
        name: CaseName("a_warm_run_over_an_unchanged_tree_leaves_the_store_as_it_was"),
        word: Word::Succeeds,
        input: || Guard::Changed(Change::Nothing, Held::AnswerAndStore),
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("an_edit_that_leaves_what_a_file_shows_the_others_keeps_the_cross_file_findings"),
        word: Word::Succeeds,
        input: || Guard::Changed(Change::Body, Held::Answer),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_file_added_before_the_others_and_one_removed_answer_as_a_run_that_keeps_nothing"),
        word: Word::Succeeds,
        input: || Guard::Changed(Change::AddedAndRemoved, Held::Answer),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_run_over_a_changed_tree_writes_the_store_again"),
        word: Word::Refuses,
        input: || Guard::Changed(Change::AddedAndRemoved, Held::AnswerAndStore),
        expect: Expect::Error(Breach::Rewritten),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let widened = |(reads, rows): (usize, usize)| (reads, u64::try_from(rows).unwrap_or(u64::MAX));
    match input {
        Guard::Warm(between) => warm(between).map(widened),
        Guard::Handover(handed) => handover(handed),
        Guard::Damaged(damage) => damaged(damage).map(widened),
        Guard::Changed(change, held) => changed(change, held).map(widened),
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
