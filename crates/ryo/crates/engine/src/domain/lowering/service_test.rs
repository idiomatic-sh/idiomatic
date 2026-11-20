//! The `lowering` service's cases: no rule lowers to an empty rule set; until the lowering expresses a step, the first rule is
//! refused at its first step.

use super::errors::LoweringError;
use super::service::lowered;
use super::types::{PlanNode, PlanShape, SelectedRule, SelectedRules};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::named;
use crate::domain::programs::service::accepted;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

type Input = usize;
/// How many rules the lowered set runs.
type Value = usize;
type Failure = LoweringError;

/// The rows of this part's cases: how many copies of the tree rule are lowered.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_rule_lowers_to_an_empty_rule_set"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the_first_rule_is_refused_at_its_first_step"),
        word: Word::Refuses,
        input: || 2,
        expect: Expect::Error(LoweringError::Unlowerable {
            rule: SelectedRuleOrdinal(0),
            shape: PlanShape::Step(PlanNode(0)),
        }),
    },
];

fn subject(count: Input) -> Result<Value, Failure> {
    let plan = named("one tree rule");
    let rules: Vec<SelectedRule<'_>> = (0..count)
        .map(|rule| SelectedRule {
            rule: SelectedRuleOrdinal(u32::try_from(rule).unwrap_or(u32::MAX)),
            plan: &plan,
        })
        .collect();
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
    };
    lowered(SelectedRules(&rules), &kinds).map(|programs| accepted(&programs).len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
