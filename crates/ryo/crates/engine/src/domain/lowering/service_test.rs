//! The `lowering` service's cases: each scene's plans lowered, and the pack said in lines (`fixtures::described`): the
//! definitions with their patterns, the tables with their stages, the rules with their steps, each with its cost class. Every
//! expected line is what a run printed, read and found right.

use super::fixtures::described::{code_census, described};
use super::fixtures::named;
use super::service::{lowered, lowered_at};
use super::placement::types::SampleStage;
use super::types::{SelectedRule, SelectedRules};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use crate::domain::statistics::types::Statistics;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The scenes whose plans a row lowers together, one rule each, and whether the load holds a first run's statistics.
type Input = (&'static [&'static str], SampleStage);
/// The pack in lines, then its codes by count, then each refused rule: one text, a line each.
type Value = Cow<'static, str>;
type Failure = String;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_rule_lowers_to_an_empty_pack"),
        word: Word::Succeeds,
        input: || (&[], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed("codes: {\"End\": 1}")),
    },
];

fn subject((scenes, stage): Input) -> Result<Value, Failure> {
    let plans: Vec<_> = scenes.iter().map(|scene| named(scene)).collect();
    let rules: Vec<SelectedRule<'_>> = plans
        .iter()
        .zip(0..)
        .map(|(plan, rule)| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan,
            bindings: RuleBindings(&[]),
        })
        .collect();
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
    };
    let statistics = Statistics::default();
    let lowered = match stage {
        SampleStage::Absent => lowered(SelectedRules(&rules), &statistics, &kinds),
        SampleStage::First | SampleStage::Second => {
            lowered_at(SelectedRules(&rules), stage, &statistics, &kinds)
        }
    };
    let mut lines = described(&lowered.pack);
    // The pack is written by the straight encoding, the holder of the invariant: its codes, by count.
    match crate::domain::encoding::service::encoded(&lowered.pack) {
        Ok(code) => lines.push(format!("codes: {:?}", code_census(&code))),
        Err(error) => lines.push(format!("not encoded: {error}")),
    }
    lines.extend(lowered.refused.iter().map(|refused| {
        format!("refused rule {}: {}", refused.rule.0, refused.refusal)
    }));
    Ok(Cow::Owned(lines.join("\n")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Prints every scene's pack: `cargo nextest run ... printed --no-capture`. Not a case; the cases above hold its lines.
#[test]
fn printed() {
    let scenes: &[Input] = &[
        (&["one tree rule"], SampleStage::Absent),
        (&["one tree rule", "one tree rule"], SampleStage::Absent),
        (&["unbound correlated sub-plan"], SampleStage::Absent),
        (&["a product within a file"], SampleStage::Absent),
        (&["the words a", "the words b"], SampleStage::Absent),
        (&["one table, two readers"], SampleStage::Absent),
        (&["one table, two readers"], SampleStage::First),
        (&["as-of latest before"], SampleStage::Absent),
        (&["as-of tied events"], SampleStage::Absent),
        (&["as-of tied events"], SampleStage::First),
        (&["pairs within one group"], SampleStage::Absent),
        (&["join with an empty side"], SampleStage::Absent),
        (&["join empty by its keys"], SampleStage::First),
        (&["one tree rule", "over the host's rows"], SampleStage::Absent),
    ];
    for scene in scenes {
        println!("== {scene:?}");
        println!("{}", subject(*scene).unwrap_or_default());
    }
    for kind in ryo_values::domain::query_operators::types::OperationKind::ALL {
        let plan = super::fixtures::growth::growth_plan(*kind);
        let bindings = super::fixtures::pending::growth_bindings(&plan);
        let rules = [SelectedRule {
            rule: SelectedRuleOrdinal(0),
            plan: &plan,
            bindings: RuleBindings(&bindings),
        }];
        let kinds = KindVocabulary {
            language: LanguageName("walking".into()),
            kinds: Vec::new(),
        };
        for stage in [SampleStage::Absent, SampleStage::First] {
            let lowered = lowered_at(SelectedRules(&rules), stage, &Statistics::default(), &kinds);
            let said = match (lowered.refused.first(), crate::domain::encoding::service::encoded(&lowered.pack)) {
                (Some(refused), _) => format!("refused: {}", refused.refusal),
                (None, Ok(code)) => format!("encoded: {:?}", code_census(&code)),
                (None, Err(error)) => format!("NOT ENCODED: {error}\n{}", described(&lowered.pack).join("\n")),
            };
            println!("growth {kind:?} {stage:?}: {said}");
        }
    }
}
