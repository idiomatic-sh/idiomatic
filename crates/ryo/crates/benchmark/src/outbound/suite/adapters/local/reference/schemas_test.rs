//! What the reference certificate document accepts and refuses, one row of `CASES` for each input.
use super::{ClaimedDiscrepancy, ReferenceDiscrepancySchema};
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = &'static str;
type Value = usize;
type Failure = ();

/// The rows of this part's cases: a certificate with one evidence row is read, and anything the shape does not name is refused.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a certificate with language evidence is read"),
        word: Word::Succeeds,
        input: || "reason='r'\nevidence=[{kind='language-spec',path='p',sha256='a',language-version='1',target-configuration='t',version-rationale='v'}]\nidentity = {tool='toy',version='1',observed-executable-sha256='a',profile-sha256='a',settings-sha256='a',source-sha256='a',reference-output-sha256='a',intended-output-sha256='a'}\n",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a field the certificate does not name is refused"),
        word: Word::Refuses,
        input: || "reason='r'\nsurprise=1\nevidence=[]\nidentity = {tool='toy',version='1',observed-executable-sha256='a',profile-sha256='a',settings-sha256='a',source-sha256='a',reference-output-sha256='a',intended-output-sha256='a'}\n",
        expect: Expect::Error(()),
    },
];

/// Decodes one certificate and counts its evidence rows.
fn subject(text: Input) -> Result<Value, Failure> {
    let schema: ReferenceDiscrepancySchema = toml::from_str(text).map_err(|_refused| ())?;
    ClaimedDiscrepancy::try_from(schema)
        .map(|claim| claim.evidence.len())
        .map_err(|_refused| ())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
