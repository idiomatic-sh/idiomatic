//! What `up` records and what `down` drops and lowers.

use super::service::{accepting, lowering};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::judgement::types::{Judgement, Resolution, Resolved};
use crate::domain::operations::types::{Count, Day, Provenance, Reason, Record};
use crate::domain::renames::types::Carried;
use crate::domain::violations::types::{
    Fingerprint, Fingerprinted, Id, Key, Level, LineNumber, MessageText, Place, RepoPath, RuleId,
    Standing, Suppression, ToolId, Violation,
};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Two new violations of a key already accepted once.
    Accept,
    /// One new violation with no fingerprint.
    Unkeyed,
    /// One record fully resolved, one partly.
    Lower,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("new_violations_of_an_accepted_key_raise_its_count"),
        word: Word::Succeeds,
        input: || Question::Accept,
        expect: Expect::Value(std::borrow::Cow::Borrowed("k:3:why")),
    },
    Case {
        name: CaseName("a_violation_with_no_key_cannot_be_accepted"),
        word: Word::Succeeds,
        input: || Question::Unkeyed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("unkeyed 1")),
    },
    Case {
        name: CaseName("a_resolved_record_is_dropped_and_a_partly_resolved_one_lowered"),
        word: Word::Succeeds,
        input: || Question::Lower,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "dropped gone lowered part:1 +part -gone",
        )),
    },
];

fn key(path: &str) -> Key {
    Key {
        tool: ToolId("t".into()),
        rule: RuleId("r".into()),
        path: RepoPath(path.into()),
        level: Level::Error,
        standing: Standing::Reported,
        fingerprint: Fingerprint(1),
    }
}

fn record(path: &str, count: u32) -> Record {
    Record {
        id: Id(path.into()),
        key: key(path),
        count: Count(count),
        message: MessageText("m".into()),
        since_rev: Provenance::Unknown,
        since: Day("2026-10-01".into()),
        reasons: vec![Reason("old".into())],
    }
}

fn violation(fingerprint: Fingerprinted) -> Violation {
    Violation {
        path: RepoPath("k".into()),
        place: Place::Line(LineNumber(1)),
        tool: ToolId("t".into()),
        rule: RuleId("r".into()),
        level: Level::Error,
        suppression: Suppression::Unsuppressed,
        message: MessageText("m".into()),
        fingerprint,
    }
}

fn id_of(key: &Key) -> Id {
    Id(key.path.0.clone())
}

fn told(question: Input) -> String {
    let since = Day("2026-10-19".into());
    let reason = Reason("why".into());
    match question {
        Question::Accept => {
            let known = violation(Fingerprinted::Known(Fingerprint(1)));
            let made = accepting(
                &[record("k", 1)],
                &[known.clone(), known],
                &Provenance::Unknown,
                &since,
                Some(&reason),
                &id_of,
            );
            made.records
                .iter()
                .map(|record| format!("{}:{}:{}", record.id.0, record.count.0, record.reasons[0].0))
                .collect::<Vec<_>>()
                .join(" ")
        }
        Question::Unkeyed => {
            let made = accepting(
                &[],
                &[violation(Fingerprinted::Unknown)],
                &Provenance::Unknown,
                &since,
                Some(&reason),
                &id_of,
            );
            format!("unkeyed {}", made.unkeyed.0)
        }
        Question::Lower => {
            let resolved = |path: &str, count: u32, remaining: u32| Resolved {
                record: record(path, count + remaining),
                count: Count(count),
                remaining: Count(remaining),
                resolution: Resolution::NotReported,
            };
            let outcome = Judgement {
                resolved: vec![resolved("gone", 1, 0), resolved("part", 1, 1)],
                ..Judgement::default()
            };
            let down = lowering(&outcome, &Carried::default());
            let dropped = down
                .dropped
                .iter()
                .map(|dropped| format!("dropped {}", dropped.record.id.0));
            let lowered = down.lowered.iter().map(|lowered| {
                format!("lowered {}:{}", lowered.record.id.0, lowered.record.count.0)
            });
            let added = down
                .changes
                .added
                .iter()
                .map(|record| format!("+{}", record.id.0));
            let removed = down
                .changes
                .removed
                .iter()
                .map(|record| format!("-{}", record.id.0));
            dropped
                .chain(lowered)
                .chain(added)
                .chain(removed)
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

fn subject(input: Input) -> Value {
    std::borrow::Cow::Owned(told(input))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
