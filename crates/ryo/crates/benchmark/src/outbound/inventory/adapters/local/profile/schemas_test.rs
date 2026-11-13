//! A projection that changes nothing stages the profile's own bytes; one that does writes the projected profile.
use super::projected;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::types::{ApplicabilityText, ProfileProjection};
use ryo_benchmark_contracts::domain::documents::types::text::value::RuleName;
use ryo_benchmark_contracts::domain::requests::types::{ParserConfigurationName, ParserSelection};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// A singleton profile already in its projected form.
const SINGLETON: &str = "tool = \"lint\"\ndefaults = [\"A1\"]\nparser-configuration = \"target=1\"\n\n[rule-applicability]\nA1 = \"always\"\n";

/// A profile's text and the rule it is projected to.
type Input = (&'static str, &'static str);
/// `unchanged`, or the projected profile.
type Value = Cow<'static, str>;
/// The refusal.
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a profile the projection leaves unchanged is staged as its bytes"),
        word: Word::Succeeds,
        input: || (SINGLETON, "A1"),
        expect: Expect::Value(Cow::Borrowed("unchanged")),
    },
    Case {
        name: CaseName("a cohort profile is projected to the one rule, deferred and pending dropped"),
        word: Word::Succeeds,
        input: || ("tool = \"lint\"\ndefaults = [\"A1\", \"B1\"]\nparser-configuration = \"target=1\"\n\n[deferred]\nlater = [\"B1\"]\n", "A1"),
        expect: Expect::Value(Cow::Borrowed("defaults = [\"A1\"]\nparser-configuration = \"target=1\"\ntool = \"lint\"\n\n[rule-applicability]\nA1 = \"always\"\n")),
    },
    Case {
        name: CaseName("a profile that is not TOML is refused"),
        word: Word::Refuses,
        input: || ("defaults = [", "A1"),
        expect: Expect::Error(Cow::Borrowed("TOML parse error at line 1, column 13\n  |\n1 | defaults = [\n  |             ^\nunclosed array, expected `]`\n")),
    },
];

fn subject((source, rule): Input) -> Result<Value, Failure> {
    projected(
        source,
        &ProfileProjection {
            defaults: vec![RuleName(rule.into())],
            supporting_rules: Vec::new(),
            parser_configuration: ParserSelection::Named(ParserConfigurationName("target=1".into())),
            query_modules: BTreeMap::new(),
            rule_applicability: BTreeMap::from([(RuleName(rule.into()), ApplicabilityText("always".into()))]),
        },
    )
    .map(|staged| staged.map_or(Cow::Borrowed("unchanged"), Cow::Owned))
    .map_err(|error| Cow::Owned(error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
