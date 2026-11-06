//! The unread check's cases: a kind no reader looks at is a finding with the way out written in it, and a kind marked unread is not.

use super::findings;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::parse::types::{KindName, KindSample, Sample, SampleText, Why};
use ryo_relations::domain::relations::types::KindBits;

/// What the case gives: the kinds of the language and the numbers its readers look at.
type Input = fn() -> (Vec<KindSample>, Vec<KindBits>);
/// What the check says.
type Value = &'static str;
type Failure = ();

fn kind(name: &str, bits: u16, sample: Sample) -> KindSample {
    KindSample {
        kind: KindName(name.into()),
        bits: KindBits(bits),
        sample,
    }
}

fn one_kind_nobody_reads() -> (Vec<KindSample>, Vec<KindBits>) {
    (
        vec![
            kind("Fn", 1, Sample::Node(SampleText("fn a() {}"))),
            kind("Abi", 2, Sample::Node(SampleText("extern \"C\" fn a() {}"))),
        ],
        vec![KindBits(1)],
    )
}

fn a_kind_marked_unread() -> (Vec<KindSample>, Vec<KindBits>) {
    (
        vec![kind(
            "Abi",
            2,
            Sample::Unread(Why("no relation models an ABI")),
        )],
        Vec::new(),
    )
}

fn a_kind_marked_unread_without_a_reason() -> (Vec<KindSample>, Vec<KindBits>) {
    (vec![kind("Abi", 2, Sample::Unread(Why("")))], Vec::new())
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_no_relation_reads_is_a_finding_that_says_how_to_read_or_mark_it"),
        word: Word::Succeeds,
        input: || one_kind_nobody_reads,
        expect: Expect::Value(
            "rust's parser produces `Abi` (sample: `extern \\\"C\\\" fn a() {}`), and no relation reads it. Either read it: add NodeKind::Abi to a reader's `kinds:`. Or mark it: Sample::Unread(\"<why no relation models it>\") in sample().",
        ),
    },
    Case {
        name: CaseName("a_kind_marked_unread_with_its_reason_is_no_finding"),
        word: Word::Succeeds,
        input: || a_kind_marked_unread,
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_kind_marked_unread_without_a_reason_is_a_finding"),
        word: Word::Succeeds,
        input: || a_kind_marked_unread_without_a_reason,
        expect: Expect::Value(
            "rust: NodeKind::Abi is marked Sample::Unread(\"\") with no reason: say why no relation models it",
        ),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let (kinds, read) = given();
    Ok(Box::leak(
        findings("rust", &kinds, &read).join("\n").into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
