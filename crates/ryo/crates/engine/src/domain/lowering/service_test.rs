//! The `lowering` service's cases: no rule lowers to an empty pack; until the lowering evaluates a step, each rule is refused at
//! its first step, named.

use super::errors::LoweringError;
use super::service::lowered;
use super::types::{LoadRefusal, PlanNode, SelectedRule, SelectedRules};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::{digests_of, named};
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// How many copies of the tree rule a row lowers.
type Input = u32;
/// How many rules the lowered pack runs.
type Value = usize;
type Failure = LoweringError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_rule_lowers_to_an_empty_pack"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName(
            "a_rule_is_refused_at_its_first_step_and_the_last_of_them_is_named_as_each_is",
        ),
        word: Word::Refuses,
        input: || 2,
        expect: Expect::Error(LoweringError::Refused {
            rule: SelectedRuleOrdinal(1),
            refusal: LoadRefusal::Unlowerable(PlanNode(0)),
        }),
    },
];

fn subject(count: Input) -> Result<Value, Failure> {
    let plan = named("one tree rule");
    let digests = digests_of(&plan, RuleBindings(&[]));
    let rules: Vec<SelectedRule<'_>> = (0..count)
        .map(|rule| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan: &plan,
            bindings: RuleBindings(&[]),
            digests: &digests,
        })
        .collect();
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let lowered = lowered(SelectedRules(&rules), None, &kinds);
    // Every rule the lowering refuses is answered: the row names the last, which is there only when each before it is.
    match lowered.refused.last() {
        Some(refused) if lowered.refused.len() == rules.len() => Err(LoweringError::Refused {
            rule: refused.rule,
            refusal: refused.refusal,
        }),
        Some(_) | None => Ok(lowered.pack.rules.len()),
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
