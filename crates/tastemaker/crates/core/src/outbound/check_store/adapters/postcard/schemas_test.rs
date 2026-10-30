//! What the postcard schemas read back: the answer they wrote, every text kept once.

use super::StoreSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::check_store::types::Checked;

/// What a row writes.
#[derive(Clone, Copy, Debug)]
enum Written {
    /// An answer that names one rule twice: as a rule that ran, and as a finding's rule.
    OneRuleTwice,
}

type Input = Written;
/// The texts the store keeps, and whether its answer reads back whole.
type Value = (usize, bool);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_answer_written_is_read_back_each_text_once"),
    word: Word::Succeeds,
    input: || Written::OneRuleTwice,
    expect: Expect::Value((4, true)),
}];

fn subject(written: Input) -> Value {
    use crate::domain::findings::types::{Finding, Fingerprint, Help, MessageText, Place, RuleId};
    let Written::OneRuleTwice = written;
    let rule = crate::domain::findings::service::of_rule(RuleId("p/a".into()));
    let answer = Checked {
        findings: vec![Finding {
            path: crate::domain::paths::types::RepoPath("src/a.rs".into()),
            place: Place::Whole,
            rule: rule.clone(),
            message: MessageText("a marker".into()),
            help: Help::Silent,
            fingerprint: Fingerprint(1),
        }],
        evaluated: [rule].into(),
        ..Checked::default()
    };
    let kept = StoreSchema::with(&crate::domain::settings::types::Build("b".into()), &answer);
    (kept.strings.len(), kept.checked() == Some(answer))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
