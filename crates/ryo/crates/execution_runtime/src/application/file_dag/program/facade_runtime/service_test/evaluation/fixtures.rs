//! The support of the `evaluation` cases.
use super::super::fixtures::request::{plan, rule};
use super::super::fixtures::scenario::Scenario;
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::open_file;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::observation_schedule::types::RuleSchedule;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use ryo_execution_domain::domain::rule_intent::types::{
    CaptureIntent, DiagnosticDelivery, EmitDemand, FileRuleIntent, ObservationDemand, RuleDemand,
};
use ryo_execution_domain::domain::table_cells::fixtures::{
    SMALL_SOURCE, SmallTree, rows_over, small_tree,
};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::service::ScopePartials;
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

pub(in super::super) type Value = (usize, &'static str);

type Failure = OperatingError;

pub(crate) fn subject(scenario: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
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
        ObservationIdentity::Independent,
    )?;
    let target = StepId((plan(scenario).steps.len() - 1) as u32);
    let engine = Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
        crate::application::file_dag::fixtures::operators(),
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
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
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
        ryo_execution_domain::domain::coverage::types::RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        &read,
        &mut symbols,
    )?;
    let routes = file.route_observation(
        ryo_execution_domain::domain::coverage::types::RuleIndex(0),
        target,
    )?;
    let table = match outcome.emit {
        RootOutcome::Complete(table) => table,
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => return Err(error),
        _ => return Err(OperatingError::NotRun),
    };
    // A completed target is answered by its registered operator, or, only when it is a select of a source relation,
    // by the shared walk's one source pass (the Phase 6 cohort walk, whose comparisons are the `comparing` concept's
    // one implementation). A refused target ran nothing, so the error above returns first.
    let reads_a_source = matches!(
        plan(scenario).steps[target.0 as usize].operation,
        ryo_values::domain::plans::types::Operation::Select {
            from: ryo_values::domain::plan_parts::types::Input::Relation(_),
            ..
        }
    );
    assert!(
        routes.registered_calls > 0 || (reads_a_source && file.walk_counts().passes.0 > 0),
        "a completed target is answered by its registered operator or, for a select of a source, the shared walk"
    );
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
