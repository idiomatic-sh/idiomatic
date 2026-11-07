//! Authored request assembly; every operation runs through the public execution owner.
use crate::outbound::facts::types::ResidentStability;
use crate::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::application::file_dag::derived::owner::types::DerivedAccess;
use crate::application::file_dag::program::prepared_execution::service::prepare;
use crate::domain::executable_program::types::RoutePolicy;
use crate::outbound::derived::interface::DerivedPort;
use crate::outbound::operators::interface::OperatorsPort;
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::SourceLanguage;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::EmitDemand;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::RuleDemand;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use crate::domain::prepared_request::types::ObservationIdentity;
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::{
    domain::elements::types::Elements,
    domain::symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_relations::domain::{
    relations::types::{KindBits, Offset},
    terms::types::NameText,
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    hosted::types::HostRelation,
    plans::types::Plan,
    query_bindings::types::ParameterBinding,
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::Table,
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::{cell::Cell, collections::BTreeMap, sync::Arc};

pub(super) fn published(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
    symbols: &mut Symbols,
) -> Result<Table, OperatingError> {
    published_file(plan, bindings, hosted, &[], &Elements::default(), symbols)
}

pub(super) fn published_file(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
) -> Result<Table, OperatingError> {
    let reads = Cell::new(0);
    let read = |_: Relation| {
        reads.set(reads.get() + 1);
        Err(OperatingError::NotRead)
    };
    let result = published_from(plan, bindings, hosted, source, elements, symbols, &read);
    if result.is_ok() {
        assert_eq!(
            reads.get(),
            0,
            "request-only/tree fixtures never demand a forged catalog provider"
        );
    }
    result
}

pub(super) fn published_from<'facts>(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> Result<Table, OperatingError> {
    published_observed(plan, bindings, hosted, source, elements, symbols, read)
        .map(|(table, _)| table)
}

pub(super) fn published_observed<'facts>(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> Result<(Table, crate::domain::operating::types::ExecutionObservation), OperatingError> {
    let (engine, prepared) = prepared(plan, bindings, hosted)?;
    run_prepared(&engine, &prepared, source, elements, symbols, read)
}
/// One authored request: its plans with their bindings, its rules, digests and hosted relations.
pub(super) struct Authored {
    pub(super) plans: Vec<Plan>,
    pub(super) bindings: Vec<Vec<ParameterBinding>>,
    pub(super) rules: Vec<RuleSpec>,
    pub(super) digests: Vec<Digest>,
    pub(super) hosted: Vec<HostRelation>,
    pub(super) observations: ObservationIdentity,
}
/// The application's preparation over the operators and derived views it is given.
pub(super) struct Preparation {
    operators: Arc<dyn OperatorsPort>,
    derived: DerivedAccess,
}
/// The real operators capability, as the engine composes it.
pub(super) fn operators() -> impl OperatorsPort {
    crate::outbound::operators::factory::use_case(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
            ryo_operators::outbound::matching::factory::use_case(),
        )),
    )
}
impl Preparation {
    /// The preparation over the real operators and derived views.
    pub(super) fn real() -> Self {
        Self::new(
            Arc::new(operators()),
            Arc::new(derived()),
        )
    }
    pub(super) fn new(operators: Arc<dyn OperatorsPort>, derived: Arc<dyn DerivedPort>) -> Self {
        Self {
            operators,
            derived: derived.into(),
        }
    }
    /// The program of an authored request, assembled and prepared as the engine prepares it.
    pub(super) fn prepare(&self, request: Authored) -> Result<PreparedExecutionProgram, OperatingError> {
        let compiled = crate::domain::prepared_request::service::assemble(
            request.plans,
            request.bindings,
            &request.rules,
        )?;
        prepare(
            compiled,
            &request.rules,
            &request.digests,
            &request.hosted,
            request.observations,
            Arc::clone(&self.operators),
            &self.derived,
            RoutePolicy::default(),
        )
    }
}
/// The derived capability behind its port, as the edge composes it.
pub(super) fn derived() -> impl crate::outbound::derived::interface::DerivedPort {
    crate::outbound::derived::factory::semantics()
}
/// The execution use case that begins the requests of the proofs.
pub(super) fn execution() -> ExecutionUseCaseImpl {
    ExecutionUseCaseImpl::new()
}
pub(super) fn prepared(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
) -> Result<(Preparation, PreparedExecutionProgram), OperatingError> {
    prepared_many(vec![plan], vec![bindings], hosted)
}
/// One request of several authored rules, the first named as `prepared` names its only one.
pub(super) fn prepared_many(
    plans: Vec<Plan>,
    bindings: Vec<Vec<ParameterBinding>>,
    hosted: Vec<HostRelation>,
) -> Result<(Preparation, PreparedExecutionProgram), OperatingError> {
    let engine = Preparation::real();
    let rules: Vec<RuleSpec> = (0..plans.len())
        .map(|at| {
            let id = if at == 0 {
                "authored-operator-contract".to_string()
            } else {
                format!("authored-operator-contract-{at}")
            };
            RuleSpec {
                id: RuleId(id.clone().into()),
                query: QueryBundle {
                    root: QueryDocumentSource {
                        id: QuerySourceId(id.into()),
                        text: "".into(),
                    },
                    modules: BTreeMap::new().into(),
                },
                applicability: RuleApplicability::Always,
                given: vec![],
                observations: vec![],
                spelled: RuleParameters("".into()),
                include: vec![],
                exclude: vec![],
            }
        })
        .collect();
    let digests = (0..plans.len())
        .map(|at| Digest([u8::try_from(at).unwrap_or(u8::MAX); 16]))
        .collect();
    let prepared = engine.prepare(Authored { plans, bindings, rules, digests, hosted, observations: ObservationIdentity::Independent })?;
    Ok((engine, prepared))
}
/// Repeat the supported public owner protocol over the same immutable compiled request.
pub(super) fn run_prepared<'facts>(
    _engine: &Preparation,
    prepared: &PreparedExecutionProgram,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> Result<(Table, crate::domain::operating::types::ExecutionObservation), OperatingError> {
    run_prepared_context(_engine, prepared, source, elements, symbols, read, None)
}

/// The same public owner with its real borrowed rendering capability.
pub(super) fn run_prepared_context<'facts>(
    _engine: &Preparation,
    prepared: &PreparedExecutionProgram,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
    semantic: Option<&dyn crate::domain::operating::types::SemanticLookup>,
) -> Result<(Table, crate::domain::operating::types::ExecutionObservation), OperatingError> {
    let last = super::interface::PreparedExecution::emit(prepared, RuleIndex(0))?.step;
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(source.len() as u64),
        modified: Nanoseconds(0),
    })];
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
    let mut file = request.open_file(
        FileInput {
            language: SourceLanguage::Recognized,
            resident_stability: ResidentStability::Stable,
            file: FileId(0),
            evaluation: &[],
            publication: &[],
            intents: &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }],
            source,
            elements,
            read,
            semantic,
            kinds: crate::application::file_dag::program::file_session::types::KindSupport::Absent,
        },
        symbols,
    )?;
    let outcome = file.rule(
        RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        symbols,
    );
    let mut observed = match &outcome {
        Ok(result) if matches!(result.emit, RootOutcome::Complete(_)) => {
            file.inner.route_observation(RuleIndex(0), last)?
        }
        _ => Default::default(),
    };
    // Every actual invocation workspace retains this required owner Rc. A
    // surviving workspace would therefore keep this weak owner live.
    let owner = file.inner.owner_lifetime();
    assert!(owner.upgrade().is_some());
    let finished = file.finish();
    observed.file_owner_released = owner.upgrade().is_none();
    assert!(
        observed.file_owner_released,
        "file finish drops the actual owner and all mandatory workspace references"
    );
    request.finish();
    finished?;
    let table = match outcome?.emit {
        RootOutcome::Complete(table) => table,
        RootOutcome::Failed(error) => return Err(error),
        RootOutcome::Unknown(error) => panic!(
            "expected a complete publication or exact Failed outcome, got Unknown({error:?})"
        ),
        _ => panic!("authored per-file request must publish a complete table or its exact failure"),
    };
    Ok((table, observed))
}

/// Exact typed input data, not a provider for one of the catalog relations.
pub(super) fn numbers(name: &str, width: usize, rows: &[&[i64]]) -> HostRelation {
    use ryo_relations::domain::relations::types::Integer;
    use ryo_values::domain::{
        hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRow},
        row_texts::types::Cell,
    };
    HostRelation {
        name: HostName(name.into()),
        columns: (0..width)
            .map(|index| HostColumn {
                name: HostColumnName(format!("column-{index}").into()),
                kind: HostKind::Number,
            })
            .collect(),
        rows: rows
            .iter()
            .map(|row| {
                HostRow(
                    row.iter()
                        .map(|number| Cell::Number(Integer(*number)))
                        .collect(),
                )
            })
            .collect(),
    }
}
/// Assemble explicit step data; the production compiler owns dependencies and schemas.
pub(super) fn plan(operations: Vec<ryo_values::domain::plans::types::Operation>) -> Plan {
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, StepId, StepScope},
        plans::types::{Emit, PlanStep},
    };
    let emit = Emit {
        step: StepId((operations.len() - 1) as u32),
        subject: ColumnIndex(0),
        key: vec![],
        headings: vec![],
    };
    Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        emit,
        steps: operations
            .into_iter()
            .map(|operation| PlanStep {
                scope: StepScope::PerFile,
                operation,
            })
            .collect(),
    }
}

pub(super) fn texts(name: &str, rows: &[&str]) -> HostRelation {
    use ryo_values::domain::{
        hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRow},
        row_texts::types::Cell,
    };
    HostRelation {
        name: HostName(name.into()),
        columns: vec![HostColumn {
            name: HostColumnName("text".into()),
            kind: HostKind::Text,
        }],
        rows: rows
            .iter()
            .map(|text| HostRow(vec![Cell::Text((*text).into())]))
            .collect(),
    }
}
/// Resolve published physical vocabulary; this does not evaluate an operation.
pub(super) fn text<'a>(
    cell: ryo_relations::domain::relations::types::Value,
    symbols: &'a Symbols,
) -> &'a str {
    use crate::domain::table_texts::{
        service::datum,
        types::{Datum, Texts},
    };
    let texts = Texts {
        symbols,
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    match datum(cell, &texts) {
        Datum::Text(text) => text.0,
        _ => panic!("published text cell required"),
    }
}

pub(super) fn tree_with_names() -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    enter_node(&mut builder, KindBits(3), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    leave_node(&mut builder, Offset(2));
    // A language names every kind its table holds.
    for (bits, name) in [(1, "file"), (2, "word"), (3, "group")] {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"abcd"), FileId(0)),
            KindBits(bits),
            NameText(name),
        );
    }
    (elements, symbols)
}

/// Assemble real hosted rows or complete canonical fact rows, then use the same public file owner.
/// No primitive is invoked here and no executor, pool or raw permission is constructed.
pub(super) fn table_request(
    operation: ryo_values::domain::plans::types::Operation,
    inputs: &[&Table],
    bindings: Vec<ParameterBinding>,
    symbols: &mut Symbols,
) -> Result<(Table, crate::domain::operating::types::ExecutionObservation), OperatingError> {
    table_request_context(operation, inputs, bindings, symbols, b"", None)
}

/// Typed fixture data enters the same public owner and optional real capability.
pub(super) fn table_request_context(
    operation: ryo_values::domain::plans::types::Operation,
    inputs: &[&Table],
    bindings: Vec<ParameterBinding>,
    symbols: &mut Symbols,
    source: &[u8],
    semantic: Option<&dyn crate::domain::operating::types::SemanticLookup>,
) -> Result<(Table, crate::domain::operating::types::ExecutionObservation), OperatingError> {
    let authored = table_inputs(operation, inputs, &bindings, symbols);
    let read = |relation| {
        authored
            .facts
            .iter()
            .find(|(actual, _)| *actual == relation)
            .map(|(_, table)| SourceRows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let (engine, prepared) = prepared(authored.plan, bindings, authored.hosted)?;
    run_prepared_context(
        &engine,
        &prepared,
        source,
        &Elements::default(),
        symbols,
        &read,
        semantic,
    )
}

pub(super) struct AuthoredRows {
    pub plan: Plan,
    pub hosted: Vec<HostRelation>,
    pub facts: Vec<(Relation, Table)>,
}
/// Only fixture input and plan assembly; callers execute through the real inbound API.
pub(super) fn table_inputs(
    mut operation: ryo_values::domain::plans::types::Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
    symbols: &mut Symbols,
) -> AuthoredRows {
    use ryo_relations::domain::relations::{
        service::kinds,
        types::{CellKind, Flag, Offset, Span, Value},
    };
    use ryo_values::domain::{
        hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRow},
        plan_parts::types::{ColumnIndex, HostedNumber, Input, ParameterName, StepId},
        plans::types::Operation,
        query_bindings::types::{ParameterType, PlanParameter},
        row_texts::types::Cell as HostCell,
    };
    let mut operations = vec![];
    let mut sources = vec![];
    let mut hosted = vec![];
    let mut facts = vec![];
    for (at, table) in inputs.iter().enumerate() {
        if table
            .kinds
            .iter()
            .all(|kind| matches!(kind, CellKind::Number | CellKind::Symbol | CellKind::File))
        {
            let columns = table
                .kinds
                .iter()
                .enumerate()
                .map(|(n, kind)| HostColumn {
                    name: HostColumnName(format!("column-{n}").into()),
                    kind: match kind {
                        CellKind::Number => HostKind::Number,
                        CellKind::Symbol => HostKind::Text,
                        CellKind::File => HostKind::File,
                        _ => unreachable!(),
                    },
                })
                .collect();
            let rows = (0..table.count.0)
                .map(|index| {
                    let width = table.kinds.len();
                    let row = &table.cells[index * width..(index + 1) * width];
                    HostRow(
                        row.iter()
                            .map(|cell| match *cell {
                                Value::Number(n) => HostCell::Number(n),
                                Value::Symbol(_) => HostCell::Text(text(*cell, symbols).into()),
                                Value::File(_) => HostCell::File("operator-contract.py".into()),
                                _ => unreachable!(),
                            })
                            .collect(),
                    )
                })
                .collect();
            operations.push(Operation::Hosted {
                relation: HostedNumber(hosted.len() as u32),
            });
            hosted.push(HostRelation {
                name: HostName(format!("input-{at}").into()),
                columns,
                rows,
            });
        } else {
            let (relation, columns) = match table.kinds.as_slice() {
                [CellKind::Node] => (
                    if at == 2 {
                        Relation::Identifier
                    } else {
                        Relation::Function
                    },
                    vec![0],
                ),
                [CellKind::Node, CellKind::Node] => (Relation::Resolves, vec![0, 1]),
                [CellKind::Node, CellKind::Node, CellKind::File] => {
                    (Relation::Resolves, vec![0, 1, 4])
                }
                [CellKind::Node, CellKind::Number] => (Relation::SemanticChild, vec![0, 2]),
                [
                    CellKind::Node,
                    CellKind::Node,
                    CellKind::Number,
                    CellKind::Symbol,
                    CellKind::File,
                ] => (Relation::SemanticChild, vec![0, 1, 2, 3, 4]),
                [
                    CellKind::Number,
                    CellKind::Symbol,
                    CellKind::Node,
                    CellKind::Span,
                    CellKind::Flag,
                    CellKind::File,
                ] => {
                    if at == 0 {
                        (Relation::FormatParse, vec![6, 1, 0, 12, 8, 11])
                    } else {
                        (Relation::BraceFormatField, vec![1, 2, 0, 9, 6, 8])
                    }
                }
                [CellKind::Number, CellKind::Flag, CellKind::Symbol] => {
                    (Relation::FormatParse, vec![6, 8, 1])
                }
                [CellKind::Span] => {
                    if at == 0 {
                        (Relation::SourceLocation, vec![2])
                    } else {
                        (Relation::StringLiteral, vec![4])
                    }
                }
                [CellKind::Symbol, CellKind::Span] => (Relation::StringLiteral, vec![2, 4]),
                [CellKind::File, CellKind::Span] => (Relation::SourceLocation, vec![1, 2]),
                [CellKind::Span, CellKind::File] => (Relation::SourceLocation, vec![2, 1]),
                [
                    CellKind::Node,
                    CellKind::Symbol,
                    CellKind::File,
                    CellKind::Span,
                ] => (
                    if at == 0 {
                        Relation::Function
                    } else {
                        Relation::Identifier
                    },
                    vec![0, 1, 2, 3],
                ),

                _ => panic!("fixture must supply an actual schema for its typed data"),
            };
            for (&column, kind) in columns.iter().zip(&table.kinds) {
                assert_eq!(
                    kinds(relation)[column as usize],
                    *kind,
                    "fixture projection must follow its actual catalog"
                );
            }
            assert!(
                !facts.iter().any(|(old, _)| *old == relation),
                "distinct facts cannot masquerade as one provider snapshot"
            );
            let word = ryo_language::domain::symbols::service::intern_derived(
                symbols,
                ByteSlice(b""),
                NameText("fixture"),
            );
            let mut supplied = Table {
                kinds: kinds(relation).to_vec(),
                ..Table::default()
            };
            for row in table.cells.chunks(table.kinds.len()) {
                let mut cells = kinds(relation)
                    .iter()
                    .map(|kind| match kind {
                        CellKind::Node => {
                            Value::Node(ryo_relations::domain::relations::service::node_id(
                                FileId(0),
                                ryo_relations::domain::relations::types::Position(0),
                            ))
                        }
                        CellKind::Symbol => Value::Symbol(word),
                        CellKind::File => Value::File(FileId(0)),
                        CellKind::Number => {
                            Value::Number(ryo_relations::domain::relations::types::Integer(0))
                        }
                        CellKind::Span => Value::Span(Span {
                            start: Offset(0),
                            end: Offset(0),
                        }),
                        CellKind::Flag => Value::Flag(Flag::Lowered),
                    })
                    .collect::<Vec<_>>();
                for (&column, &value) in columns.iter().zip(row) {
                    cells[column as usize] = value;
                }
                supplied.cells.extend(cells);
                supplied.count.0 += 1;
            }
            facts.push((relation, supplied));
            operations.push(Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: columns.into_iter().map(ColumnIndex).collect(),
            });
        }
        sources.push(Input::Step(StepId((operations.len() - 1) as u32)));
    }
    let mut at = 0;
    ryo_values::domain::plans::service::each_input(&mut operation, &mut |input| {
        *input = *sources
            .get(at)
            .expect("authored fixture supplies every required input");
        at += 1;
    });
    operations.push(operation);
    let mut plan = plan(operations);
    plan.parameters = bindings
        .iter()
        .enumerate()
        .map(|(at, binding)| PlanParameter {
            name: ParameterName(format!("parameter-{at}").into()),
            kind: match binding {
                ParameterBinding::Number(_) => ParameterType::Number,
                ParameterBinding::Text(_) => ParameterType::Text,
                ParameterBinding::Numbers(_) => ParameterType::Numbers,
                ParameterBinding::Texts(_) => ParameterType::Texts,
                ParameterBinding::TextSets(_) => ParameterType::TextSets,
                ParameterBinding::Tables(_) => ParameterType::Tables,
                ParameterBinding::Flag(_) => ParameterType::Flag,
            },
        })
        .collect();
    AuthoredRows {
        plan,
        hosted,
        facts,
    }
}
