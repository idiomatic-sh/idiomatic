//! Join assertions enter the actual compiled file owner and its real semantic port.
use super::*;
use crate::domain::{
    operating::types::ExecutionObservation,
    table_cells::types::TreeRows,
    table_model::{collections::representations::types::RepresentationKind, service::materialize},
};
use ryo_relations::{
    domain::relations::{
        service::to_bits,
        types::{ColumnsView, FactColumn, NodeId},
    },
    domain::terms::types::Symbol,
};
use ryo_values::domain::{
    stored_facts::types::KeptWords,
    table_words::{
        service::{admit_compact, encode_table, pack},
        types::{CompactTable, DecodeMode, WordSlice},
    },
};
use std::cell::Cell as Counter;

/// Canonical provider data and its physical encoding; no evaluator or admission factory.
struct Provider<'a> {
    relation: Relation,
    table: Table,
    columns: Vec<FactColumn>,
    compact: Option<CompactTable>,
    tree: Option<TreeRows<'a>>,
    representation: RepresentationKind,
}
impl Provider<'_> {
    fn rows(&self) -> Rows<'_> {
        if let Some(tree) = self.tree {
            return Rows::Tree(tree);
        }
        match self.representation {
            RepresentationKind::Stored => Rows::Stored(ColumnsView {
                kinds: kinds(self.relation),
                columns: &self.columns,
            }),
            RepresentationKind::Compact => Rows::Compact(self.compact.as_ref().unwrap()),
            _ => Rows::Computed(&self.table),
        }
    }
}
fn defaults(kind: CellKind, row: usize) -> Cell {
    match kind {
        CellKind::Node => Cell::Node(NodeId(row as u64)),
        CellKind::Symbol => Cell::Symbol(Symbol(0)),
        CellKind::File => Cell::File(FileId(0)),
        CellKind::Span => Cell::Span(Span {
            start: Offset(0),
            end: Offset(0),
        }),
        CellKind::Number => Cell::Number(Integer(0)),
        CellKind::Flag => Cell::Flag(Flag::Lowered),
    }
}
/// Project a truthful catalog schema onto the fixture's logical tuple. Entry and
/// SequenceElement each have multiple numeric/node columns and actual Flag fields.
fn provider<'a>(
    rows: Rows<'a>,
    side: usize,
    steps: &mut Vec<PlanStep>,
) -> (Provider<'a>, PlanInput) {
    if let Rows::Tree(tree) = rows {
        let relation = match tree.form {
            TreeForm::Elements => Relation::SyntaxTree,
            TreeForm::Edges => Relation::SyntaxChild,
        };
        return (
            Provider {
                relation,
                table: Table::default(),
                columns: vec![],
                compact: None,
                tree: Some(tree),
                representation: RepresentationKind::Tree,
            },
            PlanInput::Relation(relation),
        );
    }
    let representation = match &rows {
        Rows::Stored(_) => RepresentationKind::Stored,
        Rows::Compact(_) => RepresentationKind::Compact,
        _ => RepresentationKind::Computed,
    };
    let mut logical = Table::default();
    materialize(&rows, &mut logical, &mut Vec::new());
    let relation = if side == 0 {
        Relation::Entry
    } else {
        Relation::SequenceElement
    };
    let schema = kinds(relation);
    let mut used = Vec::new();
    let projection = logical
        .kinds
        .iter()
        .map(|kind| {
            let at = schema
                .iter()
                .enumerate()
                .find(|(at, candidate)| *candidate == kind && !used.contains(at))
                .map(|(at, _)| at)
                .expect("canonical catalog has the fixture's typed columns");
            used.push(at);
            ColumnIndex(at as u32)
        })
        .collect::<Vec<_>>();
    let mut cells = Vec::new();
    for row in 0..logical.count.0 {
        let mut output = schema
            .iter()
            .copied()
            .map(|kind| defaults(kind, row))
            .collect::<Vec<_>>();
        for (column, value) in projection
            .iter()
            .zip(&logical.cells[row * logical.kinds.len()..(row + 1) * logical.kinds.len()])
        {
            output[column.0 as usize] = *value;
        }
        cells.extend(output);
    }
    let table = Table {
        kinds: schema.to_vec(),
        cells,
        count: logical.count,
    };
    let columns = (0..schema.len())
        .map(|column| {
            FactColumn(
                table
                    .cells
                    .chunks_exact(schema.len())
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect();
    let compact = if representation == RepresentationKind::Compact {
        let mut words = ryo_relations::domain::relations::types::Words::default();
        encode_table(&table, &[], &mut words);
        let (table, names) = admit_compact(
            Arc::new(KeptWords(pack(WordSlice(&words.0)))),
            &mut Vec::new(),
            DecodeMode::Complete,
        )
        .unwrap();
        assert!(names.is_empty());
        Some(table)
    } else {
        None
    };
    let input = PlanInput::Step(StepId(steps.len() as u32));
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: PlanInput::Relation(relation),
            comparisons: vec![],
            kept: projection,
        },
    });
    (
        Provider {
            relation,
            table,
            columns,
            compact,
            tree: None,
            representation,
        },
        input,
    )
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum SourceAttempt {
    Available(usize),
    Refused(usize, OperatingError),
}
struct Observation {
    result: Result<Table, OperatingError>,
    prefix: Vec<Vec<Cell>>,
    reads: [usize; 2],
    attempts: Vec<SourceAttempt>,
    work: ExecutionObservation,
    counts: (u64, u64, u64),
    fed: Vec<crate::domain::operator::types::RowsFed>,
    instructions: usize,
    native_prepared: bool,
    column_prepared: bool,
    frontiers: Vec<Table>,
    frontier_errors: Vec<OperatingError>,
    affected: Vec<Option<FileId>>,
}
/// Fixture behavior belongs to real external source/sink dependencies only.
#[derive(Default)]
struct Faults {
    left: Option<OperatingError>,
    right: Option<OperatingError>,
    sink_after: Option<usize>,
    optional_limit: Option<usize>,
    partials: Vec<crate::domain::coverage::types::Partial>,
}
fn invoke(
    operation: &Operation,
    rows: [Rows<'_>; 2],
    symbols: &mut Symbols,
    source: &[u8],
    delivery: bool,
    port: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    faults: Faults,
) -> Result<Observation, OperatingError> {
    let mut steps = Vec::new();
    let (left, l) = provider(rows[0].clone(), 0, &mut steps);
    let (right, r) = provider(rows[1].clone(), 1, &mut steps);
    let mut operation = operation.clone();
    let Operation::Join {
        left: from,
        right: other,
        ..
    } = &mut operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    *from = l;
    *other = r;
    let target = steps.len();
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation,
    });
    let mut request = plan(Scenario::ProjectRetry);
    request.steps = steps;
    request.emit.step = StepId(target as u32);
    execute(
        request,
        [left, right],
        symbols,
        source,
        delivery,
        port,
        faults,
    )
}
fn execute(
    request: Plan,
    providers: [Provider<'_>; 2],
    symbols: &mut Symbols,
    source: &[u8],
    delivery: bool,
    port: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    faults: Faults,
) -> Result<Observation, OperatingError> {
    let target = request.emit.step;
    let has_frontier = !request.frontier.is_empty();
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            plans: vec![request],
            bindings: vec![vec![]],
            observations: vec![vec![]],
            schedule: RuleSchedule::Independent,
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let program = prepared.program();
    let logical = program.rules()[0].original_steps[target.0 as usize]
        .as_ref()
        .unwrap()
        .logical;
    let region = program.logical_regions()[logical.0];
    let instructions = program.regions()[region.0].instructions.len();
    let native_prepared = program.regions()[region.0].native_positions.is_some();
    let column_prepared = program.regions()[region.0].column.is_some();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(port)),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("join-owner.py".into()),
            size: FileSize(source.len() as u64),
            modified: Nanoseconds(0),
        })],
    ));
    let reads = [Counter::new(0), Counter::new(0)];
    let attempts = std::cell::RefCell::new(Vec::new());
    let read = |relation| {
        let side = providers
            .iter()
            .position(|p| p.relation == relation)
            .ok_or(OperatingError::NotRead)?;
        reads[side].set(reads[side].get() + 1);
        let fault = if side == 0 {
            &faults.left
        } else {
            &faults.right
        };
        if let Some(error) = fault {
            attempts
                .borrow_mut()
                .push(SourceAttempt::Refused(side, error.clone()));
            return Err(error.clone());
        }
        attempts.borrow_mut().push(SourceAttempt::Available(side));
        Ok(providers[side].rows())
    };
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &faults.partials,
            publication: &faults.partials,
        },
        &[FileRuleIntent::Run {
            diagnostic: if delivery {
                DiagnosticDelivery::Forwarded
            } else {
                DiagnosticDelivery::Collected
            },
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }],
        services,
        source,
        true,
        symbols,
        None,
        None,
    )?;
    if let Some(limit) = faults.optional_limit {
        file.owner.derived.set_optional_limit(limit)?;
    }
    let mut prefix = Vec::new();
    let mut affected = vec![];
    let result = if delivery {
        file.emit_rows(RuleIndex(0), &read, symbols, &mut |row| {
            if faults.sink_after == Some(prefix.len()) {
                return Err(OperatingError::NotRun);
            }
            prefix.push(row.to_vec());
            Ok(())
        })
        .map(|_| Table {
            cells: prefix.iter().flatten().copied().collect(),
            count: RowCount(prefix.len()),
            ..Table::default()
        })
    } else {
        let outcome = file.rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )?;
        match outcome.emit {
            RootOutcome::Complete(table) => Ok(table),
            RootOutcome::Partial {
                table,
                affected: partial,
            } => {
                affected = partial;
                Ok(table)
            }
            RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
            _ => Err(OperatingError::NotRun),
        }
    };
    let work = file.route_observation(RuleIndex(0), target)?;
    let mut frontier_errors = vec![];
    let frontiers = if has_frontier {
        let mut arena = vec![];
        let mut places = vec![];
        let mut words = ryo_relations::domain::relations::types::Words::default();
        file.capture_frontiers(
            RuleIndex(0),
            ryo_values::domain::stored_facts::types::Bucket(0),
            &read,
            symbols,
            super::super::super::super::facade::CaptureContext {
                file: FileId(0),
                rule_digest: Digest([0; 16]),
                worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                retained: crate::domain::finish::types::Retained {
                    arena: crate::domain::finish::types::Arena(&mut arena),
                    places: &mut places,
                },
                words: &mut words,
            },
        )?
        .into_iter()
        .filter_map(|capture| {
            let capture = match capture.outcome {
                Ok(capture) => capture,
                Err(error) => {
                    frontier_errors.push(error);
                    return None;
                }
            };
            let (compact, _) = admit_compact(
                Arc::new(KeptWords(capture.words)),
                &mut vec![],
                DecodeMode::Complete,
            )
            .expect("actual captured words decode");
            let mut table = Table::default();
            materialize(&Rows::Compact(&compact), &mut table, &mut vec![]);
            Some(table)
        })
        .collect()
    } else {
        vec![]
    };
    let counts = file.owner.derived.counts();
    let fed = file.counts(RuleIndex(0))?.fed.to_vec();
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(
        owner.derived.working_bytes(),
        0,
        "actual invocation resources released by normal owner shutdown"
    );
    assert_eq!(
        owner.derived.owned_bytes(),
        0,
        "finished file releases owned index storage"
    );
    Ok(Observation {
        result,
        prefix,
        reads: reads.map(|n| n.get()),
        attempts: attempts.into_inner(),
        work,
        counts,
        fed,
        instructions,
        native_prepared,
        column_prepared,
        frontiers,
        frontier_errors,
        affected,
    })
}
fn real_port() -> Arc<dyn crate::outbound::operators::interface::OperatorsPort> {
    Arc::new(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new())
}
fn operation(
    left: &[u32],
    right: &[u32],
    output: &[(Side, u32)],
    filter: Vec<ryo_values::domain::plan_parts::types::Comparison>,
    view: &[u32],
) -> Operation {
    Operation::Join {
        left: PlanInput::Relation(Relation::Entry),
        right: PlanInput::Relation(Relation::SequenceElement),
        left_on: left.iter().copied().map(ColumnIndex).collect(),
        right_on: right.iter().copied().map(ColumnIndex).collect(),
        output: output
            .iter()
            .map(|(side, column)| JoinColumn {
                side: *side,
                column: ColumnIndex(*column),
            })
            .collect(),
        filter,
        view: view.iter().copied().map(ColumnIndex).collect(),
        reach: None,
    }
}
fn made(
    op: &Operation,
    rows: [Rows<'_>; 2],
    symbols: &mut Symbols,
    source: &[u8],
) -> Result<Table, OperatingError> {
    invoke(
        op,
        rows,
        symbols,
        source,
        false,
        real_port(),
        Faults::default(),
    )?
    .result
}
#[cfg(test)]
mod capability;
#[cfg(test)]
mod cases;

#[cfg(test)]
mod lifecycle;
#[cfg(test)]
mod populations;

#[cfg(test)]
mod column_chain;

#[cfg(test)]
mod diagnostics;
#[cfg(test)]
mod storage;

#[cfg(test)]
mod direct;

#[cfg(test)]
mod generations;
