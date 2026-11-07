//! The scenario subject: the prepared file program run over one small tree.
use super::request::{plan, rule};
use super::scenario::Scenario;
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::open_file;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::application::file_dag::program::protocol::types::{
    CaptureIntent, DiagnosticDelivery, EmitDemand, FileRuleIntent, ObservationDemand, RootOutcome,
    RuleDemand,
};
use crate::application::file_dag::scope::ScopePartials;
use crate::domain::file_program::types::RuleSchedule;
use crate::domain::operating::errors::OperatingError;
use crate::domain::positions::service_test::{SMALL_SOURCE, SmallTree, rows_over, small_tree};
use crate::domain::prepared_request::types::{Compiled, ObservationIdentity};
use crate::domain::table_model::types::Rows;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::service::{kinds, node_id};
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span,
};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;
type Input = Scenario;
type Value = (usize, &'static str);
type Failure = OperatingError;
pub(crate) fn subject(scenario: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan(scenario)],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent, crate::domain::executable_program::types::RoutePolicy::default(),
    )?;
    let target = StepId((plan(scenario).steps.len() - 1) as u32);
    let target_region = {
        let program = prepared.program();
        let logical = program.rules()[0].original_steps[target.0 as usize]
            .as_ref()
            .unwrap()
            .logical;
        program.logical_regions()[logical.0]
    };
    let expects_native = matches!(
        scenario,
        Scenario::NativeAntijoin
            | Scenario::NativeUnion
            | Scenario::NativeJoinSteps
            | Scenario::NativeJoinTree
    );
    if expects_native {
        assert!(
            crate::domain::executable_program::service::native_positions(
                &prepared.program().regions()[target_region.0].route
            )
            .is_some(),
            "this case requires the compiled native-position strategy to be installed"
        );
    }
    if matches!(scenario, Scenario::OrdinaryFunctionJoin) {
        assert!(
            crate::domain::executable_program::service::native_positions(
                &prepared.program().regions()[target_region.0].route
            )
            .is_none(),
            "the non-tree RHS must decline native-position installation"
        );
    }
    let engine = Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
    )));
    let services = Arc::new(ProgramServices::new(
        engine,
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("native-parity.py".into()),
            size: FileSize(SMALL_SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })],
    ));
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
        ryo_values::domain::files::types::SourceView(SMALL_SOURCE),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )?;
    let function = Table {
        kinds: kinds(Relation::Function).to_vec(),
        cells: kinds(Relation::Function)
            .iter()
            .copied()
            .map(|kind| match kind {
                CellKind::Node => Cell::Node(node_id(FileId(0), Position(2))),
                CellKind::Symbol => Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Cell::Number(Integer(0)),
                CellKind::Flag => Cell::Flag(Flag::Lowered),
            })
            .collect(),
        count: RowCount(1),
    };
    let read = |relation| match relation {
        Relation::SyntaxTree => Ok(Rows::Tree(rows_over(&elements))),
        Relation::Function => Ok(Rows::Computed(&function)),
        _ => Err(OperatingError::NotRead),
    };
    let outcome = file.rule(
        crate::domain::coverage::types::RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        &read,
        &mut symbols,
    )?;
    let routes = file.route_observation(crate::domain::coverage::types::RuleIndex(0), target)?;
    if expects_native {
        assert!(
            routes.native_positions > 0,
            "published rows must come from actual native-position execution; ordinary parity alone is insufficient"
        );
        assert_eq!(
            routes.registered_calls, 0,
            "the target did not dispatch through ordinary registration"
        );
    }
    if matches!(scenario, Scenario::OrdinaryFunctionJoin) {
        assert_eq!(routes.native_positions, 0);
        assert!(
            routes.registered_calls > 0,
            "a declined native shape must execute the real registered operator"
        );
    }
    let table = match outcome.emit {
        RootOutcome::Complete(table) => table,
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => return Err(error),
        _ => return Err(OperatingError::NotRun),
    };
    let shown = table
        .cells
        .iter()
        .map(|cell| match cell {
            Cell::Node(node) => node.0.to_string(),
            Cell::Number(Integer(number)) => number.to_string(),
            Cell::Symbol(symbol)
                if matches!(
                    scenario,
                    Scenario::DistinctKinds
                        | Scenario::KeptKinds
                        | Scenario::NativeAntijoin
                        | Scenario::NativeUnion
                ) =>
            {
                ryo_language::domain::symbols::service::text_of(
                    &symbols,
                    ryo_language::domain::symbols::types::ByteSlice(SMALL_SOURCE),
                    *symbol,
                )
                .map_or_else(|| "?".to_owned(), |text| text.0.to_owned())
            }
            Cell::Symbol(_) | Cell::Name(_) | Cell::File(_) | Cell::Span(_) | Cell::Flag(_) => {
                "_".to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    Ok((table.count.0, Box::leak(shown.into_boxed_str())))
}
