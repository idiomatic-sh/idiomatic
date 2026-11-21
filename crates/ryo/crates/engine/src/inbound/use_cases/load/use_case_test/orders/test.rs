//! The load's cases over the order of a selection: five copies of the tree rule, some refused each for its own cause (by
//! the lowering, by the encoding, by the executor's check), selected in each of the 120 orders through a lowering that
//! writes each rule as its row says. Every order loads the same rules and answers the same refusals: a refused rule
//! takes no other rule with it, whatever is refused beside it and wherever it stands.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::lowering::fixtures::{digests_of, named};
use crate::domain::lowering::types::{
    LoadRefusal, LoweredRules, PlanNode, RefusedRule, SelectedRule, SelectedRules,
};
use crate::domain::programs::service::accepted;
use crate::domain::steps::types::{AnchorKinds, PlacedStep, ProgramStep, Register, StepProgram};
use crate::inbound::use_cases::load::fixtures::kinds;
use crate::inbound::use_cases::load::service::sequenced;
use ryo_relations::domain::relations::types::{KindBits, Relation};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::BTreeSet;

/// What the lowering makes of a rule.
#[derive(Clone, Copy, Debug)]
enum Written {
    /// Its anchor alone: every word runs.
    Whole,
    /// Nothing: the lowering refuses it, naming step 7 so that its refusal is told from the others'.
    Refused,
    /// Its anchor twice: the encoding's own fault.
    Faulty,
    /// Its anchor and the children of a node, a word the loop does not run.
    Unrun,
}

type Input = [Written; 5];
/// How many orders were loaded, what each said (one answer when every order says the same), and the fewest and the most
/// lowerings an order took.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "three_rules_load_beside_one_the_lowering_refuses_and_one_the_executor_cannot_run_in_every_order",
        ),
        word: Word::Succeeds,
        input: || {
            [
                Written::Whole,
                Written::Refused,
                Written::Whole,
                Written::Unrun,
                Written::Whole,
            ]
        },
        expect: Expect::Value(Cow::Borrowed(
            "120 orders: runs {0, 2, 4}; refused [1: step PlanNode(7) cannot be evaluated; 3: step PlanNode(0) cannot be evaluated]; lowered 2 to 2 times",
        )),
    },
    Case {
        name: CaseName(
            "three_rules_load_beside_one_the_encoding_refuses_and_one_the_executor_cannot_run_in_every_order",
        ),
        word: Word::Succeeds,
        input: || {
            [
                Written::Whole,
                Written::Faulty,
                Written::Whole,
                Written::Unrun,
                Written::Whole,
            ]
        },
        expect: Expect::Value(Cow::Borrowed(
            "120 orders: runs {0, 2, 4}; refused [1: step PlanNode(0) cannot be evaluated; 3: step PlanNode(0) cannot be evaluated]; lowered 3 to 3 times",
        )),
    },
    Case {
        name: CaseName(
            "three_rules_load_beside_one_the_lowering_refuses_and_one_the_encoding_refuses_in_every_order",
        ),
        word: Word::Succeeds,
        input: || {
            [
                Written::Refused,
                Written::Whole,
                Written::Whole,
                Written::Whole,
                Written::Faulty,
            ]
        },
        expect: Expect::Value(Cow::Borrowed(
            "120 orders: runs {1, 2, 3}; refused [0: step PlanNode(7) cannot be evaluated; 4: step PlanNode(0) cannot be evaluated]; lowered 2 to 2 times",
        )),
    },
    Case {
        name: CaseName(
            "two_rules_load_beside_two_the_encoding_refuses_and_one_the_executor_cannot_run_in_every_order",
        ),
        word: Word::Succeeds,
        input: || {
            [
                Written::Faulty,
                Written::Whole,
                Written::Faulty,
                Written::Unrun,
                Written::Whole,
            ]
        },
        expect: Expect::Value(Cow::Borrowed(
            "120 orders: runs {1, 4}; refused [0: step PlanNode(0) cannot be evaluated; 2: step PlanNode(0) cannot be evaluated; 3: step PlanNode(0) cannot be evaluated]; lowered 4 to 4 times",
        )),
    },
    Case {
        name: CaseName("five_rules_each_refused_for_its_own_cause_load_nothing_in_every_order"),
        word: Word::Succeeds,
        input: || {
            [
                Written::Unrun,
                Written::Refused,
                Written::Faulty,
                Written::Unrun,
                Written::Faulty,
            ]
        },
        expect: Expect::Value(Cow::Borrowed(
            "120 orders: runs {}; refused [0: step PlanNode(0) cannot be evaluated; 1: step PlanNode(7) cannot be evaluated; 2: step PlanNode(0) cannot be evaluated; 3: step PlanNode(0) cannot be evaluated; 4: step PlanNode(0) cannot be evaluated]; lowered 4 to 4 times",
        )),
    },
];

/// A lowered rule as `written` says; none of a rule the lowering refuses.
fn lowered_rule(rule: SelectedRuleOrdinal, written: Written) -> Option<LoweredRule> {
    let anchor = || PlacedStep {
        point: None,
        step: ProgramStep::EachKind {
            relation: Relation::SyntaxTree,
            kinds: AnchorKinds(Box::new([KindBits(2)])),
            out: Register(0),
        },
    };
    let children = PlacedStep {
        point: None,
        step: ProgramStep::Children {
            relation: Relation::SyntaxTree,
            parent: Register(0),
            field: None,
            out: Register(0),
        },
    };
    let steps = match written {
        Written::Whole => vec![anchor()],
        Written::Refused => return None,
        Written::Faulty => vec![anchor(), anchor()],
        Written::Unrun => vec![anchor(), children],
    };
    Some(LoweredRule {
        rule,
        node: NodeDigest([u8::try_from(rule.0).unwrap_or(u8::MAX); 16]),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: StepProgram(steps.into()),
        }],
        columns: RuleColumns::default(),
    })
}

/// Every order of the five rules.
fn orders() -> Vec<Vec<u32>> {
    let mut found = vec![Vec::new()];
    for _ in 0..5 {
        found = found
            .iter()
            .flat_map(|order: &Vec<u32>| {
                (0..5_u32)
                    .filter(|rule| !order.contains(rule))
                    .map(|rule| order.iter().copied().chain([rule]).collect())
                    .collect::<Vec<Vec<u32>>>()
            })
            .collect();
    }
    found
}

fn subject(written: Input) -> Result<Value, Failure> {
    let plan = named("one tree rule");
    let digests = digests_of(&plan, RuleBindings(&[]));
    let of = |rule: SelectedRuleOrdinal| {
        let place = usize::try_from(rule.0).unwrap_or(usize::MAX);
        written.get(place).copied().unwrap_or(Written::Whole)
    };
    let lowerings = Cell::new(0_usize);
    let lower = |selected: SelectedRules<'_>| {
        lowerings.set(lowerings.get().saturating_add(1));
        let each = selected.0.iter().map(|rule| rule.rule);
        LoweredRules {
            pack: LoweredPack {
                rules: each
                    .clone()
                    .filter_map(|rule| lowered_rule(rule, of(rule)))
                    .collect(),
                ..LoweredPack::default()
            },
            refused: each
                .filter(|rule| matches!(of(*rule), Written::Refused))
                .map(|rule| RefusedRule {
                    rule,
                    refusal: LoadRefusal::Unlowerable(PlanNode(7)),
                })
                .collect(),
        }
    };
    let orders = orders();
    let mut answers = Vec::new();
    for order in &orders {
        let rules: Vec<SelectedRule<'_>> = order
            .iter()
            .map(|rule| SelectedRule {
                rule: SelectedRuleOrdinal(*rule),
                plan: &plan,
                bindings: RuleBindings(&[]),
                digests: &digests,
            })
            .collect();
        lowerings.set(0);
        let loaded = sequenced(SelectedRules(&rules), None, &kinds(), &lower).map_err(|_| ())?;
        let refused: Vec<String> = loaded
            .refused
            .iter()
            .map(|rule| format!("{:?}: {}", rule.rule.0, rule.refusal))
            .collect();
        // The programs are in the order their rules were selected in: the rules that run are compared as a set.
        let runs: BTreeSet<u32> = accepted(&loaded.programs)
            .iter()
            .map(|rule| rule.0)
            .collect();
        answers.push((
            format!("runs {runs:?}; refused [{}]", refused.join("; ")),
            lowerings.get(),
        ));
    }
    // What the orders said, each different answer once; and how many times each lowered.
    let said: BTreeSet<&str> = answers.iter().map(|(said, _)| said.as_str()).collect();
    let said: Vec<&str> = said.into_iter().collect();
    let lowered: BTreeSet<usize> = answers.iter().map(|(_, lowered)| *lowered).collect();
    Ok(Cow::Owned(format!(
        "{} orders: {}; lowered {} to {} times",
        orders.len(),
        said.join(" | "),
        lowered.first().copied().unwrap_or(0),
        lowered.last().copied().unwrap_or(0)
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
