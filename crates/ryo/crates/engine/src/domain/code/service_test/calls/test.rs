//! Every per-row meaning, a source search once for each search it may be, written into a `Call` word and into a `StageCall`
//! word by its code, its regular expression and its flag, and read back as the meaning it was.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::deciding::service::{call, call_of};
use crate::domain::code::deciding::types::CallWord;
use crate::domain::code::filling::service::{stage_call, stage_call_of};
use crate::domain::code::small_codes::service::{
    meaning_code, meaning_flag, meaning_of, meaning_regex,
};
use crate::domain::code::types::{CodeWord, PoolAt};
use crate::domain::steps::calls::fixtures::searches;
use crate::domain::steps::types::{CallMeaning, RegexId};
use std::borrow::Cow;

/// What a row writes and reads back.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Every meaning through both call words.
    EveryMeaning,
    /// The flag each search writes.
    SearchFlags,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_meaning_reads_back_through_both_call_words"),
        word: Word::Succeeds,
        input: || Scenario::EveryMeaning,
        expect: Expect::Value(Cow::Borrowed(
            "19 of 19 through Call, 19 of 19 through StageCall",
        )),
    },
    Case {
        name: CaseName("each_search_has_its_own_flag"),
        word: Word::Succeeds,
        input: || Scenario::SearchFlags,
        expect: Expect::Value(Cow::Borrowed("0 2 6 10 14 1 3 7 11 15")),
    },
];

/// The regular expression the meanings that name one name here.
const EXPRESSION: RegexId = RegexId(0x0003_0003);

/// Every meaning, a source search once for each search it may be.
fn meanings() -> Vec<CallMeaning> {
    let mut meanings = vec![
        CallMeaning::SpanBetween,
        CallMeaning::SpanBounds,
        CallMeaning::Capture,
        CallMeaning::Replace(EXPRESSION),
        CallMeaning::Words,
        CallMeaning::Rank,
        CallMeaning::Concat,
        CallMeaning::Lower,
        CallMeaning::FitsRendered,
    ];
    meanings.extend(
        searches()
            .into_iter()
            .map(|search| CallMeaning::SourceMatches(EXPRESSION, search)),
    );
    meanings
}

/// Whether a meaning written through `write` reads back through `read` as the operands written and as itself.
fn reads_back(
    meaning: CallMeaning,
    write: fn(CallWord) -> CodeWord,
    read: fn(CodeWord) -> CallWord,
) -> bool {
    let wrote = CallWord {
        flag: meaning_flag(meaning),
        meaning: meaning_code(meaning),
        arguments: PoolAt(0x0001_0001),
        outputs: PoolAt(0x0002_0002),
        regex: meaning_regex(meaning).unwrap_or(RegexId(0)),
    };
    let back = read(write(wrote));
    back == wrote && meaning_of(back.meaning, back.regex, back.flag) == Some(meaning)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let meanings = meanings();
    Ok(Cow::Owned(match scenario {
        Scenario::EveryMeaning => {
            let through = |write, read| {
                meanings
                    .iter()
                    .filter(|meaning| reads_back(**meaning, write, read))
                    .count()
            };
            format!(
                "{} of {} through Call, {} of {} through StageCall",
                through(call, call_of),
                meanings.len(),
                through(stage_call, stage_call_of),
                meanings.len()
            )
        }
        Scenario::SearchFlags => searches()
            .into_iter()
            .map(|search| {
                meaning_flag(CallMeaning::SourceMatches(EXPRESSION, search))
                    .0
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(" "),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
