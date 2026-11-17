//! What the phase's one read of the project asks for, through `project_reads`: each project relation the steps of the given
//! rules read, once, in the order a step first reads it; a file relation, and a rule not given, ask for nothing.
use crate::application::file_dag::file_pass::service::pass_program;
use crate::application::file_dag::file_pass::types::PassSource;
use crate::application::file_dag::project_pass::service::project_reads;
use crate::outbound::operators::fixtures::fake::OperatorsFake;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, StepId, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use std::borrow::Cow;
use std::sync::Arc;

/// Which of the two rules the phase runs: rule 0 reads two project relations, the second twice, and a file relation; rule 1
/// reads the second project relation and a third.
#[derive(Clone, Copy, Debug)]
enum Rules {
    /// Rule 0.
    First,
    /// Rule 1, then rule 0.
    Both,
    /// None.
    Neither,
}

type Input = Rules;
/// The relations asked for, as text.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_project_relation_a_rule_reads_is_asked_for_once_in_first_read_order"),
        word: Word::Succeeds,
        input: || Rules::First,
        expect: Expect::Value(Cow::Borrowed(
            "[ModuleIdentity, ProjectNamespaceMembership]",
        )),
    },
    Case {
        name: CaseName("the_rules_reads_are_asked_for_once_across_rules"),
        word: Word::Succeeds,
        input: || Rules::Both,
        expect: Expect::Value(Cow::Borrowed(
            "[ProjectNamespaceMembership, ManifestDiagnostic, ModuleIdentity]",
        )),
    },
    Case {
        name: CaseName("a_phase_of_no_rule_asks_for_nothing"),
        word: Word::Succeeds,
        input: || Rules::Neither,
        expect: Expect::Value(Cow::Borrowed("[]")),
    },
];

/// A select of everything `relation` holds, after the files.
fn read(relation: Relation) -> PlanStep {
    PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: StepInput::Relation(relation),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
    }
}

/// A plan of `steps` that emits its first.
fn plan(steps: Vec<PlanStep>) -> Arc<Plan> {
    Arc::new(Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps,
    })
}

fn subject(rules: Input) -> Result<Value, Failure> {
    let source = PassSource {
        plans: vec![
            plan(vec![
                read(Relation::ModuleIdentity),
                read(Relation::ProjectNamespaceMembership),
                read(Relation::Function),
                read(Relation::ProjectNamespaceMembership),
            ]),
            plan(vec![
                read(Relation::ProjectNamespaceMembership),
                read(Relation::ManifestDiagnostic),
            ]),
        ],
        bindings: vec![Vec::new(), Vec::new()],
        operators: Arc::new(OperatorsFake::default()),
        hosted: Vec::new(),
        ids: Vec::new(),
    };
    let pass = pass_program(&source).map_err(|_| ())?;
    let given = match rules {
        Rules::First => vec![RuleIndex(0)],
        Rules::Both => vec![RuleIndex(1), RuleIndex(0)],
        Rules::Neither => Vec::new(),
    };
    Ok(Cow::Owned(format!("{:?}", project_reads(&pass, &given))))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
