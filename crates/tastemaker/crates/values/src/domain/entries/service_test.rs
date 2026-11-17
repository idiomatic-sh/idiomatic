//! What the `entries` service answers of counts, identities and values.

use super::service::{answered, count_text, derived, identity, interpolate};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::{CaptureBinding, CaptureName, CaptureValue};
use crate::domain::entries::types::{
    Alias, Amount, Bound, Branch, ConditionName, Count, Described, EntryHeader, ParamValues,
    Value as EntryValue,
};
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Ownership, PackageName, VersionText,
};
use crate::domain::scalars::types::TextValue;
use crate::domain::selectors::types::{PathPattern, Selectors};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    Identity,
    OwnedIdentity,
    RangeText,
    WhenAnswered,
    Interpolated,
    DirectoryDerived,
}

/// What a row asks.
type Input = Question;
/// What the service answers.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_entry_with_an_alias_is_named_with_it"),
        word: Word::Succeeds,
        input: || Question::Identity,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_owned_entry_is_named_by_its_spec"),
        word: Word::Succeeds,
        input: || Question::OwnedIdentity,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_range_reads_as_at_least"),
        word: Word::Succeeds,
        input: || Question::RangeText,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_condition_that_holds_picks_then"),
        word: Word::Succeeds,
        input: || Question::WhenAnswered,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_capture_is_interpolated_into_text"),
        word: Word::Succeeds,
        input: || Question::Interpolated,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_capture_counts_as_a_capture"),
        word: Word::Succeeds,
        input: || Question::DirectoryDerived,
        expect: Expect::Value(String::new()),
    },
];

fn header(alias: Alias, ownership: Ownership) -> EntryHeader {
    EntryHeader {
        package: PackageName("p".into()),
        version: VersionText("0.1.0".into()),
        ownership,
        alias,
        description: Described::Unsaid,
        include: Selectors::default(),
        exclude: Selectors::default(),
        params: ParamValues::default(),
    }
}

fn when() -> Count {
    Count::When {
        condition: ConditionName("c".into()),
        then: Amount(1),
        otherwise: Amount(0),
    }
}

fn subject(question: Input) -> Value {
    let (said, expected) = match question {
        Question::Identity => (
            identity(
                &header(Alias::Named(DefinitionName("a".into())), Ownership::Shared),
                DefinitionKind::Spec,
                &DefinitionName("crate".into()),
            )
            .0,
            "p/crate(a)",
        ),
        Question::OwnedIdentity => (
            identity(
                &header(
                    Alias::Unnamed,
                    Ownership::Owned(DefinitionName("hex".into())),
                ),
                DefinitionKind::Template,
                &DefinitionName("errors".into()),
            )
            .0,
            "p/hex/errors",
        ),
        Question::RangeText => (
            count_text(&Count::Range {
                min: Amount(1),
                max: Bound::Unbounded,
            })
            .0,
            "at least 1",
        ),
        Question::WhenAnswered => (count_text(&answered(&when(), Branch::Then)).0, "exactly 1"),
        Question::Interpolated => (
            match interpolate(
                &EntryValue::Text(TextValue("ryo-{lang}".into())),
                &[CaptureBinding {
                    name: &CaptureName("lang".into()),
                    value: &CaptureValue("css".into()),
                }],
            ) {
                EntryValue::Text(text) => text.0.to_string(),
                other @ (EntryValue::Whole(_)
                | EntryValue::Float(_)
                | EntryValue::Flag(_)
                | EntryValue::List(_)
                | EntryValue::Table(_)) => format!("{other:?}"),
            },
            "ryo-css",
        ),
        Question::DirectoryDerived => (
            count_text(&derived(
                &[Some(&PathPattern("{x:dir}.rs".into()))],
                &std::collections::BTreeSet::new(),
            ))
            .0,
            "at least 1",
        ),
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |question| Ok(subject(question)));
}
