//! The step digests of a fixed plan, by the pack load's own function over the real digest adapter. The row is pinned: a
//! step's bytes are written through the derived `Hash` of the plan's types, so this row is what turns a toolchain change of
//! that derived byte stream, or a change of the hash, into a red test instead of statistics and packs that silently match
//! nothing.

use crate::application::pack_load::selecting::service::step_digests;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::digest::factory::blake3;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as PlanInput, LiteralText, Operand, ParameterId,
    ParameterName, PlanComparison, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::{
    ParameterBinding, ParameterType, PlanParameter, RuleBindings,
};
use ryo_values::domain::query_plans::types::{Emit, Plan};
use std::borrow::Cow;

type Input = ();
/// The digests, each as its sixteen bytes written in hexadecimal.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_fixed_two_step_plan_digests_to_the_written_bytes"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Cow::Borrowed(
        "04219e81c516e1d1706f3de0b165318d b246db0caf0b690a2a9fe828ae2f2af3",
    )),
}];

/// A step of `operation`, after the files.
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::CrossFile,
    }
}

/// Every function of the project, then those whose first column is the plan's one parameter, a text.
fn fixed_plan() -> Plan {
    let every = step(Operation::Select {
        from: PlanInput::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    });
    let of_the_parameter = step(Operation::Select {
        from: PlanInput::Step(StepId(0)),
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Parameter(ParameterId(0)),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    });
    Plan {
        observation_inputs: Vec::new(),
        parameters: vec![PlanParameter {
            name: ParameterName("word".into()),
            kind: ParameterType::Text,
        }],
        steps: vec![every, of_the_parameter],
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    }
}

fn subject((): Input) -> Result<Value, Failure> {
    let bound = [ParameterBinding::Text(LiteralText("a".into()))];
    let digests = step_digests(&blake3(), &fixed_plan(), RuleBindings(&bound));
    let written: Vec<String> = digests
        .0
        .iter()
        .map(|digest| digest.0.iter().map(|byte| format!("{byte:02x}")).collect())
        .collect();
    Ok(Cow::Owned(written.join(" ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
