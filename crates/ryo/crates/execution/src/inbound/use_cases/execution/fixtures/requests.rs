//! Authored request assembly, for the use case's own tests: every operation runs through the public execution owner.

use crate::application::file_dag::program::file_session::interface::FileExecution as _;
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use crate::inbound::use_cases::execution::interface::{ExecutionUseCase, CaptureIntent, DiagnosticDelivery, EmitDemand, FileRuleIntent, ObservationDemand, RuleDemand, RuleIndex};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use std::collections::BTreeMap;
use crate::outbound::facts::types::ResidentStability;
use crate::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use super::{derived, operators};
use crate::application::file_dag::derived::owner::types::DerivedAccess;
use crate::application::file_dag::program::prepared_execution::service::prepare;
use crate::outbound::derived::interface::DerivedPort;
use crate::outbound::operators::interface::OperatorsPort;
use crate::application::file_dag::program::file_session::types::{FileInput, SourceLanguage, SourceRows};
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, Relation, KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds};
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::Digest;
use std::cell::Cell;
use std::sync::Arc;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs::{plan, text, table_inputs};

pub(crate) fn published(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
    symbols: &mut Symbols,
) -> Result<Table, OperatingError> {
    published_file(plan, bindings, hosted, &[], &Elements::default(), symbols)
}
pub(crate) fn published_file(
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
pub(crate) fn published_from<'facts>(
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
pub(crate) fn published_observed<'facts>(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> Result<(Table, ryo_execution_domain::domain::operating::fixtures::ExecutionObservation), OperatingError> {
    let (engine, prepared) = prepared(plan, bindings, hosted)?;
    run_prepared(&engine, &prepared, source, elements, symbols, read)
}
/// One authored request: its plans with their bindings, its rules, digests and hosted relations.
pub(crate) struct Authored {
    pub(crate) plans: Vec<Plan>,
    pub(crate) bindings: Vec<Vec<ParameterBinding>>,
    pub(crate) rules: Vec<RuleSpec>,
    pub(crate) digests: Vec<Digest>,
    pub(crate) hosted: Vec<HostRelation>,
    pub(crate) observations: ObservationIdentity,
}
/// The application's preparation over the operators and derived views it is given.
pub(crate) struct Preparation {
    operators: Arc<dyn OperatorsPort>,
    derived: DerivedAccess,
}
impl Preparation {
    /// The preparation over the real operators and derived views.
    pub(crate) fn real() -> Self {
        Self::new(
            Arc::new(operators()),
            Arc::new(derived()),
        )
    }
    pub(crate) fn new(operators: Arc<dyn OperatorsPort>, derived: Arc<dyn DerivedPort>) -> Self {
        Self {
            operators,
            derived: derived.into(),
        }
    }
    /// The program of an authored request, assembled and prepared as the engine prepares it.
    pub(crate) fn prepare(&self, request: Authored) -> Result<PreparedExecutionProgram, OperatingError> {
        let compiled = ryo_execution_domain::domain::prepared_request::service::assemble(
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
            Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
        )
    }
}
/// The execution use case that begins the requests of the proofs.
pub(crate) fn execution() -> ExecutionUseCaseImpl {
    ExecutionUseCaseImpl::new()
}
pub(crate) fn prepared(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
) -> Result<(Preparation, PreparedExecutionProgram), OperatingError> {
    prepared_many(vec![plan], vec![bindings], hosted)
}
/// One request of several authored rules, the first named as `prepared` names its only one.
pub(crate) fn prepared_many(
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
    let prepared = engine.prepare(Authored {
        plans,
        bindings,
        hosted,
        rules,
        digests,
        observations: ObservationIdentity::Independent,
    })?;
    Ok((engine, prepared))
}
/// Repeat the supported public owner protocol over the same immutable compiled request.
pub(crate) fn run_prepared<'facts>(
    _engine: &Preparation,
    prepared: &PreparedExecutionProgram,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> Result<(Table, ryo_execution_domain::domain::operating::fixtures::ExecutionObservation), OperatingError> {
    run_prepared_context(_engine, prepared, source, elements, symbols, read, None)
}
/// The same public owner with its real borrowed rendering capability.
pub(crate) fn run_prepared_context<'facts>(
    _engine: &Preparation,
    prepared: &PreparedExecutionProgram,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
    semantic: Option<&dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
) -> Result<(Table, ryo_execution_domain::domain::operating::fixtures::ExecutionObservation), OperatingError> {
    let last = crate::application::file_dag::program::prepared_execution::interface::PreparedExecution::emit(prepared, RuleIndex(0))?.step;
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
            source: ryo_values::domain::files::types::SourceView(source),
            elements,
            read,
            semantic: match semantic {
                Some(lookup) => crate::application::file_dag::program::file_session::types::SemanticSupport::Supplied(lookup),
                None => crate::application::file_dag::program::file_session::types::SemanticSupport::Absent,
            },
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
pub(crate) fn numbers(name: &str, width: usize, rows: &[&[i64]]) -> HostRelation {
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
pub(crate) fn texts(name: &str, rows: &[&str]) -> HostRelation {
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
pub(crate) fn tree_with_names() -> (Elements, Symbols) {
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
pub(crate) fn table_request(
    operation: ryo_values::domain::plans::types::Operation,
    inputs: &[&Table],
    bindings: Vec<ParameterBinding>,
    symbols: &mut Symbols,
) -> Result<(Table, ryo_execution_domain::domain::operating::fixtures::ExecutionObservation), OperatingError> {
    table_request_context(operation, inputs, bindings, symbols, b"", None)
}
/// Typed fixture data enters the same public owner and optional real capability.
pub(crate) fn table_request_context(
    operation: ryo_values::domain::plans::types::Operation,
    inputs: &[&Table],
    bindings: Vec<ParameterBinding>,
    symbols: &mut Symbols,
    source: &[u8],
    semantic: Option<&dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
) -> Result<(Table, ryo_execution_domain::domain::operating::fixtures::ExecutionObservation), OperatingError> {
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
