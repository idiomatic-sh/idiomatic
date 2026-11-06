//! The chain runner that feeds each route through the real owner.
use super::super::super::*;
use super::sources::Sources;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures;
use crate::domain::executable_program::fusion::types::Fusibility;
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::ParameterName;
use ryo_values::domain::plans::service::{each_input, inputs_of, named};
use ryo_values::domain::query_bindings::types::{ParameterType, PlanParameter};
use ryo_values::domain::query_operators::types::OperationKind;
/// The shape a fixture's operation is compiled in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Chain {
    /// The operation alone, emitted.
    Alone,
    /// An authored row consumer after the operation.
    Downstream,
    /// A pure identity projection on every input (or after a source), fused into its producer.
    Fused,
    /// The same program, with a second reader on every producer so no projection may fuse.
    Held,
}
/// What one compiled run decided and published.
pub(crate) struct Probe {
    /// The fusion classification of each inserted projection, in program order.
    pub(crate) continuations: Vec<Option<Fusibility>>,
    pub(crate) outcome: Result<(Table, Symbols), OperatingError>,
}
pub(crate) fn execute_chain(
    kind: OperationKind,
    chain: Chain,
    operators: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
) -> Probe {
    let mut continuations = Vec::new();
    let outcome = run_chain(kind, chain, operators, &mut continuations);
    Probe {
        continuations,
        outcome,
    }
}
pub(crate) fn execute_fixture(
    kind: OperationKind,
    downstream: bool,
    operators: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
) -> Result<(Table, Symbols), OperatingError> {
    execute_chain(
        kind,
        if downstream {
            Chain::Downstream
        } else {
            Chain::Alone
        },
        operators,
    )
    .outcome
}
pub(crate) fn identity(width: usize) -> Vec<ColumnIndex> {
    (0..width as u32).map(ColumnIndex).collect()
}
pub(crate) fn run_chain(
    kind: OperationKind,
    chain: Chain,
    operators: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    continuations: &mut Vec<Option<Fusibility>>,
) -> Result<(Table, Symbols), OperatingError> {
    let mut case = fixtures::fixture(kind);
    let fusing = matches!(chain, Chain::Fused | Chain::Held);
    let mut inserted: Vec<usize> = Vec::new();
    if ryo_language::domain::names::service::name_count(&case.symbols).0 == 0 {
        query_intern_derived(
            &mut (&mut case.symbols).into(),
            ByteSlice(fixtures::SOURCE),
            NameText(""),
        );
    }
    let mut sources = Sources::default();
    let mut operation = case.operation.clone();
    if kind == OperationKind::Hosted {
        let relation = sources.snapshot(&case.inputs[0], &case.symbols);
        operation = Operation::Hosted { relation };
    } else {
        let declarations = named(&inputs_of(&operation)).to_vec();
        assert_eq!(declarations.len(), case.inputs.len());
        let installed = case
            .inputs
            .iter()
            .map(|table| sources.input(table, &case.symbols))
            .collect::<Vec<_>>();
        let installed = if fusing {
            let continued = installed
                .iter()
                .zip(&case.inputs)
                .map(|(input, table)| {
                    inserted.push(sources.steps.len());
                    sources.push(Operation::Project {
                        from: *input,
                        columns: identity(table.kinds.len()),
                        distinct: RepeatedRows::Kept,
                    })
                })
                .collect::<Vec<_>>();
            if chain == Chain::Held {
                for input in &installed {
                    sources.push(Operation::Select {
                        from: *input,
                        comparisons: vec![],
                        kept: vec![],
                    });
                }
            }
            continued
        } else {
            installed
        };
        each_input(&mut operation, &mut |input| {
            let at = declarations
                .iter()
                .position(|declared| declared == input)
                .expect("declared fixture input");
            *input = installed[at];
        });
    }
    let target = sources.steps.len();
    let produced = sources.push(operation);
    let mut emit = target;
    if chain == Chain::Downstream {
        sources.push(Operation::Extend {
            from: PlanInput::Step(StepId(target as u32)),
            values: vec![],
        });
        emit = target + 1;
    }
    if fusing && inserted.is_empty() {
        // A request-owned source has no input to continue: continue its own rows.
        inserted.push(sources.steps.len());
        emit = sources.steps.len();
        sources.push(Operation::Project {
            from: produced,
            columns: identity(case.expected[0].len()),
            distinct: RepeatedRows::Kept,
        });
        if chain == Chain::Held {
            sources.push(Operation::Select {
                from: produced,
                comparisons: vec![],
                kept: vec![],
            });
        }
    }
    let parameters = if case.bindings.is_empty() {
        vec![]
    } else {
        vec![PlanParameter {
            name: ParameterName("fixture".into()),
            kind: if kind == OperationKind::Rows {
                ParameterType::Tables
            } else {
                ParameterType::TextSets
            },
        }]
    };
    let plan = Plan {
        parameters,
        observation_inputs: vec![],
        observations: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(emit as u32),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps: sources.steps,
    };
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![case.bindings],
        },
        &[rule()],
        &[Digest([0; 16])],
        &sources.hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let program = prepared.program();
    for step in &inserted {
        continuations.push(match program.rules()[0].original_steps.get(*step) {
            Some(Ok(origin)) => program.fusibility(origin.logical),
            _ => None,
        });
    }
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(operators)),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("operation-route.py".into()),
            size: FileSize(fixtures::SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })],
    ));
    let semantic = fixtures::KnownSyntax;
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(fixtures::SOURCE),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        &mut case.symbols,
        Some(&semantic),
        None,
    )?;
    let read = |relation| {
        sources
            .providers
            .iter()
            .find(|(source, _)| *source == relation)
            .map(|(_, table)| Rows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let outcome = file.rule(
        RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        &read,
        &mut case.symbols,
    )?;
    let result = match outcome.emit {
        RootOutcome::Complete(table) => Ok(table),
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
        _ => Err(OperatingError::NotRun),
    };
    file.finish()?;
    let actual = result?;
    assert!(actual.count.0 > 0, "nonempty actual {kind:?} owner route");
    let rows = actual
        .cells
        .chunks_exact(actual.kinds.len())
        .map(<[_]>::to_vec)
        .collect::<Vec<_>>();
    fixtures::assert_expected(&case.expected, &rows, &case.symbols);
    Ok((actual, case.symbols))
}
