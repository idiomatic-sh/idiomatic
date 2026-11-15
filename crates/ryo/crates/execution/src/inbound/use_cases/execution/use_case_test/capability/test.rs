//! Recursive producers and roots run through the injected operators port, and its refusal is the rule's terminal fault.
use super::super::super::fixtures;
use super::super::super::fixtures::requests::{Authored, Preparation};
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::{EmittedRoot, RuleAfter};
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
        plan_parts::types::{ColumnIndex, HostedNumber, Input as Source, RepeatedRows, StepId},
        plans::types::Operation,
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
        stored_facts::types::Digest,
    },
};
use std::sync::Arc;
/// Whether the injected operators refuse every node they are asked to run.
#[derive(Clone, Copy, Debug)]
enum Refusal {
    /// They run every node.
    None,
    /// They refuse every node with a shape mismatch.
    Run,
}
/// The operators port the rule runs through, refusing as `refusal` says.
#[derive(Debug)]
struct InjectedOperators {
    refusal: Refusal,
}
impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for InjectedOperators {
    fn plan(
        &self,
        program: &ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram<'_>,
    ) -> Result<
        ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        ryo_operators::inbound::use_cases::run::fixtures::planned(program)
    }
    fn run(
        &self,
        plan: &ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        node: ryo_operators::domain::physical_plan::types::PlanNodeId,
        buffers: &mut ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers,
        inputs: &dyn ryo_operators::domain::physical_plan::types::PlanInputs,
        resources: &mut dyn ryo_operators::domain::physical_plan::types::Resources,
    ) -> Result<(), ryo_operators::domain::contract::types::PortableOperatorFault> {
        match self.refusal {
            Refusal::None => ryo_operators::inbound::use_cases::run::fixtures::ran(
                plan, node, buffers, inputs, resources,
            ),
            Refusal::Run => Err(PortableOperatorFault::ShapeMismatch),
        }
    }
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        fixtures::operators().operators(call)
    }
}
/// What a rule's root emitted on one demand.
#[derive(Debug, PartialEq, Eq)]
enum Emitted {
    Complete(Cow<'static, [Cell]>),
    Failed(OperatingError),
    /// Any other root outcome: partial, skipped, unknown, no query or deferred.
    Other,
}
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;
type Input = Refusal;
type Value = Emitted;
type Failure = OperatingError;
fn subject(refusal: Input) -> Result<Emitted, Failure> {
    let port = Arc::new(InjectedOperators { refusal });
    let engine = Preparation::new(port);
    let mut plan = inputs::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Hosted {
            relation: HostedNumber(1),
        },
        Operation::Union {
            left: Source::Step(StepId(0)),
            right: Source::Step(StepId(1)),
        },
        Operation::Project {
            from: Source::Step(StepId(2)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
    ]);
    // An authored identity boundary keeps the union a genuine recursive producer.
    plan.frontier.push(Source::Step(StepId(2)));
    let prepared = engine.prepare(Authored {
        plans: vec![plan],
        bindings: vec![vec![]],
        rules: vec![RuleSpec {
            id: RuleId("injected-semantics".into()),
            query: QueryBundle {
                root: QueryDocumentSource {
                    id: QuerySourceId("injected-semantics".into()),
                    text: "".into(),
                },
                modules: Default::default(),
            },
            applicability: RuleApplicability::Always,
            given: vec![],
            observations: vec![],
            spelled: RuleParameters("".into()),
            include: vec![],
            exclude: vec![],
        }],
        digests: vec![Digest([0; 16])],
        hosted: vec![
            fixtures::requests::numbers("left", 1, &[&[1], &[1], &[2]]),
            fixtures::requests::numbers("right", 1, &[&[2], &[3]]),
        ],
        observations: ObservationIdentity::Independent,
    })?;
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("injected.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::requests::execution().execution(&prepared, &corpus, None.into());
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let mut no_provider = fixtures::TableFacts::new(&[]);
    let ran = fixtures::requests::run_file(
        &request,
        fixtures::requests::RunFile {
            source: b"",
            elements: &elements,
            intents: &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }],
            semantic: None,
        },
        &mut symbols,
        &mut no_provider,
    )
    .rules
    .into_iter()
    .next()
    .ok_or(OperatingError::NotRun)?;
    let emitted = match (ran.emit, ran.after) {
        (EmittedRoot::Delivered(RootOutcome::Complete(table)), _) => {
            Emitted::Complete(table.cells.into())
        }
        (EmittedRoot::Delivered(RootOutcome::Failed(error)), _) => Emitted::Failed(error),
        (_, RuleAfter::Stopped(stop)) => {
            return Err(fixtures::facts::operating(stop));
        }
        (EmittedRoot::Delivered(_) | EmittedRoot::Withheld, _) => Emitted::Other,
    };
    request.finish();
    Ok(emitted)
}
const NUMBERS: Emitted = Emitted::Complete(Cow::Borrowed(&[
    Cell::Number(Integer(1)),
    Cell::Number(Integer(2)),
    Cell::Number(Integer(3)),
]));
const REFUSED: Emitted = Emitted::Failed(OperatingError::ShapeMismatch);
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("recursive_producer_and_root_use_the_injected_operators"),
        word: Word::Succeeds,
        input: || Refusal::None,
        expect: Expect::Value(NUMBERS),
    },
    Case {
        name: CaseName("a_refusal_of_the_injected_operators_is_the_rule_s_terminal_fault"),
        word: Word::Succeeds,
        input: || Refusal::Run,
        expect: Expect::Value(REFUSED),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
