//! The `engine` port's fake runs the port's contract, and what is its own: it accepts the lowering fixture's one tree rule, finds
//! its rows per word, keeps the accepted rules in its pack, and refuses or faults as it is told.

use super::super::contract::{self, ContractFault, found_in};
use super::{EngineFake, no_kinds};
use crate::outbound::engine::interface::EnginePort as _;
use ryo_engine::domain::lowering::fixtures::named;
use ryo_engine::domain::lowering::types::{SelectedRule, SelectedRules};
use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_relations::domain::relations::types::Count;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// What the fake is asked.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The one tree rule, scanned over a file of two words.
    Rows,
    /// The one tree rule, written to a pack and read back.
    Pack,
    /// A plan the fake does not accept.
    OtherPlan,
    /// The plan over the lowering's budget.
    OverBudget,
    /// The one tree rule, scanned over a file whose `syntax-error` rows the fixture language fills in part.
    PartialFile,
}

type Input = Scenario;
/// How many rows the fake found, or how many rules its pack read back.
type Value = Count;
/// The fault of the port's answer, as it prints.
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_one_tree_rule_finds_a_row_and_a_frontier_row_for_each_word"),
        word: Word::Succeeds,
        input: || Scenario::Rows,
        expect: Expect::Value(Count(4)),
    },
    Case {
        name: CaseName("a_pack_of_the_fake_reads_back_the_rule_it_accepted"),
        word: Word::Succeeds,
        input: || Scenario::Pack,
        expect: Expect::Value(Count(1)),
    },
    Case {
        name: CaseName("another_plan_is_refused_at_its_first_step"),
        word: Word::Refuses,
        input: || Scenario::OtherPlan,
        expect: Expect::Error(Cow::Borrowed(
            "Load(Unlowerable { rule: SelectedRuleOrdinal(0), shape: Step(PlanNode(0)) })",
        )),
    },
    Case {
        name: CaseName("the_plan_over_budget_is_refused_as_over_budget"),
        word: Word::Refuses,
        input: || Scenario::OverBudget,
        expect: Expect::Error(Cow::Borrowed(
            "Load(Budget { rule: SelectedRuleOrdinal(0) })",
        )),
    },
    Case {
        name: CaseName("a_relation_filled_in_part_faults_the_rule"),
        word: Word::Refuses,
        input: || Scenario::PartialFile,
        expect: Expect::Error(Cow::Borrowed(
            "Scan(Partial { rule: SelectedRuleOrdinal(0), relation: SyntaxError, cause: PartialCause(\"a question the fixture cannot read\"), missing: Any })",
        )),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    asked(scenario).map_err(|fault| Cow::Owned(format!("{fault:?}")))
}

/// What the fake answers a scenario.
fn asked(scenario: Input) -> Result<Value, ContractFault> {
    let (fake, kinds, digest) = (EngineFake::default(), no_kinds(), PackDigest([7; 32]));
    let plan = named(match scenario {
        Scenario::Rows | Scenario::Pack | Scenario::PartialFile => "one tree rule",
        Scenario::OtherPlan => "unbound correlated sub-plan",
        Scenario::OverBudget => "program over budget",
    });
    let rule = SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan: &plan,
    };
    let rules = fake
        .load(LoadSource::Plans(SelectedRules(&[rule])), &kinds)
        .map_err(ContractFault::Load)?;
    match scenario {
        Scenario::Pack => {
            let pack = fake.encoded(&rules, digest);
            fake.load(LoadSource::Pack(PackBytes(&pack.0), digest), &kinds)
                .map(|read| Count(fake.accepted(&read).len()))
                .map_err(ContractFault::Load)
        }
        Scenario::PartialFile => found_in(&fake, &rules, "one two?"),
        Scenario::Rows | Scenario::OtherPlan | Scenario::OverBudget => {
            found_in(&fake, &rules, "one two")
        }
    }
}

#[test]
fn succeeds() {
    contract::check(&EngineFake::default());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
