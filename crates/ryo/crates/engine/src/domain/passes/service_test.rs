//! The `passes` service's cases: the passes run in their one order over a pack and keep what it holds: no pass adds or drops a
//! rule, a definition or a table.

use super::service::passed;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::statistics::types::Statistics;
use crate::domain::steps::types::EmitTarget;
use crate::domain::steps::types::{PlacedStep, ProgramStep, StepOperands, StepProgram};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// How many rules the pack a row passes holds.
type Input = u32;
/// The rules, the branches and the steps the pack holds after the passes.
type Value = [usize; 3];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_passes_leave_an_empty_pack_empty"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("the_passes_keep_every_rule_branch_and_step_of_a_pack"),
        word: Word::Succeeds,
        input: || 2,
        expect: Expect::Value([2, 2, 2]),
    },
];

/// A rule of one branch of one step, which hands a row of no column over.
fn rule(ordinal: u32) -> LoweredRule {
    let rule = SelectedRuleOrdinal(ordinal);
    let step = ProgramStep::Emit {
        target: EmitTarget::Frontier(rule, InputNumber(0)),
        columns: StepOperands::default(),
    };
    LoweredRule {
        rule,
        node: NodeDigest::default(),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: StepProgram(Box::new([PlacedStep { point: None, step }])),
        }],
        columns: RuleColumns::default(),
    }
}

fn subject(rules: Input) -> Result<Value, Failure> {
    let mut pack = LoweredPack {
        rules: (0..rules).map(rule).collect(),
        ..LoweredPack::default()
    };
    passed(&mut pack, &Statistics::default());
    let branches = pack.rules.iter().map(|rule| rule.branches.len()).sum();
    let steps = pack
        .rules
        .iter()
        .flat_map(|rule| &rule.branches)
        .map(|branch| branch.program.0.len())
        .sum();
    Ok([pack.rules.len(), branches, steps])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
