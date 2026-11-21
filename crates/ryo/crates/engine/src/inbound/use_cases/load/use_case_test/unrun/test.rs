//! The load's cases over rules the executor cannot run whole: three copies of the tree rule through a lowering that
//! writes each as its row says. A rule that can reach a word the executor does not run is refused like an operation the
//! lowering does not take, with a step of its own plan, and never reaches a scan.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::lowering::fixtures::{digests_of, named};
use crate::domain::lowering::types::{LoweredRules, SelectedRule, SelectedRules};
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

/// What the lowering writes of a rule.
#[derive(Clone, Copy, Debug)]
enum Written {
    /// Its anchor alone: every word runs.
    Whole,
    /// Its anchor twice: the encoding's own fault.
    Faulty,
    /// Its anchor and the children of a node, a word the loop does not run.
    Unrun,
}

type Input = [Written; 3];
/// The rules the loaded set runs, each rule refused with why, and how many times the rules were lowered.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "rules_the_executor_cannot_run_whole_are_refused_together_and_the_others_load",
        ),
        word: Word::Succeeds,
        input: || [Written::Unrun, Written::Whole, Written::Unrun],
        expect: Expect::Value(Cow::Borrowed(
            "runs [1]; refused [0: step PlanNode(0) cannot be evaluated; 2: step PlanNode(0) cannot be evaluated]; lowered 2 times",
        )),
    },
    Case {
        name: CaseName(
            "a_rule_the_executor_cannot_run_beside_one_the_encoding_refuses_takes_no_other_rule",
        ),
        word: Word::Succeeds,
        input: || [Written::Faulty, Written::Whole, Written::Unrun],
        expect: Expect::Value(Cow::Borrowed(
            "runs [1]; refused [0: step PlanNode(0) cannot be evaluated; 2: step PlanNode(0) cannot be evaluated]; lowered 3 times",
        )),
    },
    Case {
        name: CaseName("rules_the_executor_runs_whole_all_load_in_one_lowering"),
        word: Word::Succeeds,
        input: || [Written::Whole, Written::Whole, Written::Whole],
        expect: Expect::Value(Cow::Borrowed("runs [0, 1, 2]; refused []; lowered 1 times")),
    },
];

/// A lowered rule as `written` says.
fn lowered_rule(rule: SelectedRuleOrdinal, written: Written) -> LoweredRule {
    let anchor = || PlacedStep {
        point: None,
        step: ProgramStep::EachKind {
            relation: Relation::SyntaxTree,
            kinds: AnchorKinds(Box::new([KindBits(2)])),
            out: Register(0),
        },
    };
    let call = PlacedStep {
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
        Written::Faulty => vec![anchor(), anchor()],
        Written::Unrun => vec![anchor(), call],
    };
    LoweredRule {
        rule,
        node: NodeDigest([u8::try_from(rule.0).unwrap_or(u8::MAX); 16]),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: StepProgram(steps.into()),
        }],
        columns: RuleColumns::default(),
    }
}

fn subject(written: Input) -> Result<Value, Failure> {
    let plan = named("one tree rule");
    let digests = digests_of(&plan, RuleBindings(&[]));
    let rules: Vec<SelectedRule<'_>> = (0..3)
        .map(|rule| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan: &plan,
            bindings: RuleBindings(&[]),
            digests: &digests,
        })
        .collect();
    let lowerings = Cell::new(0_usize);
    let lower = |selected: SelectedRules<'_>| {
        lowerings.set(lowerings.get().saturating_add(1));
        let of = |rule: &SelectedRule<'_>| {
            let place = usize::try_from(rule.rule.0).unwrap_or(usize::MAX);
            lowered_rule(
                rule.rule,
                written.get(place).copied().unwrap_or(Written::Whole),
            )
        };
        LoweredRules {
            pack: LoweredPack {
                rules: selected.0.iter().map(of).collect(),
                ..LoweredPack::default()
            },
            refused: Vec::new(),
        }
    };
    let loaded = sequenced(SelectedRules(&rules), None, &kinds(), &lower).map_err(|_| ())?;
    let refused: Vec<String> = loaded
        .refused
        .iter()
        .map(|rule| format!("{:?}: {}", rule.rule.0, rule.refusal))
        .collect();
    let runs: Vec<u32> = accepted(&loaded.programs)
        .iter()
        .map(|rule| rule.0)
        .collect();
    Ok(Cow::Owned(format!(
        "runs {runs:?}; refused [{}]; lowered {} times",
        refused.join("; "),
        lowerings.get()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
