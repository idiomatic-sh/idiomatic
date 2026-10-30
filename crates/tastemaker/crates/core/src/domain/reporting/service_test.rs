//! Pure settlement preserves suppression identity and ordering.
use super::service::{candidates, fingerprinted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::service::{error, identified};
use crate::domain::findings::types::{Fingerprint, PreparedIdentity, Reason, Suppression};
use crate::domain::paths::types::RepoPath;
type Input = ();
type Value = (usize, usize, Fingerprint);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("suppressed_and_standing_findings_keep_their_identities"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((1, 1, Fingerprint(7))),
}];
fn subject((): Input) -> Value {
    let first = error(
        crate::domain::findings::service::of_rule(crate::domain::findings::types::RuleId(
            "r".into(),
        )),
        RepoPath("a.txt".into()),
        "first",
    );
    let second = error(
        crate::domain::findings::service::of_rule(crate::domain::findings::types::RuleId(
            "s".into(),
        )),
        RepoPath("b.txt".into()),
        "second",
    );
    let all = candidates(
        vec![second],
        vec![Suppression {
            finding: first,
            reason: Reason("valid".into()),
        }],
    );
    let identities = identified(all.iter().map(|candidate| PreparedIdentity {
        finding: &candidate.finding,
        line: candidate.line.clone(),
    }));
    let result = fingerprinted(all, identities, &|_| Fingerprint(7));
    (
        result.standing.len(),
        result.accepted.len(),
        result.accepted[0].finding.fingerprint,
    )
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
