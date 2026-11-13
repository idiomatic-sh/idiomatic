//! Authored request assembly, for the use case's own tests: every operation runs through the public execution owner.

use super::{derived, operators};
use crate::application::derived::service::DerivedPreparation;
use crate::inbound::use_cases::execution::fixtures::TableFacts;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::{
    Arena, CaptureContext, EmittedRoot, FactsWorker, FileEnd, FileRules, FileRunInput, KindSupport,
    RequestExecution, RuleAfter, SemanticSupport, SourceLanguage,
};
use crate::inbound::use_cases::execution::interface::{
    CaptureIntent, DiagnosticDelivery, ExecutionUseCase, FileRuleIntent, ObservationDemand,
};
use crate::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use crate::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution_domain::domain::rule_intent::types::{EmitDemand, RuleDemand};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs::table_inputs;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Words, WorkerNumber};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_values::domain::stored_facts::types::{Bucket, Digest};
use std::collections::BTreeMap;
use std::sync::Arc;

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
    let mut facts = TableFacts::new(&[]);
    // Request-only and tree fixtures have no catalog provider: a run that demanded one answers the read's refusal,
    // even when it went on to publish.
    let result = published_from(plan, bindings, hosted, source, elements, symbols, &mut facts);
    if facts.reads() > 0 {
        return Err(result.err().unwrap_or(OperatingError::NotRead));
    }
    result
}
pub(crate) fn published_from(
    plan: Plan,
    bindings: Vec<ParameterBinding>,
    hosted: Vec<HostRelation>,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    facts: &mut dyn FactsWorker,
) -> Result<Table, OperatingError> {
    let (engine, prepared) = prepared(plan, bindings, hosted)?;
    run_prepared(&engine, &prepared, source, elements, symbols, facts)
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
    derived: DerivedPreparation,
}
impl Preparation {
    /// The preparation over the real operators and derived views.
    pub(crate) fn real() -> Self {
        Self::new(Arc::new(operators()), Arc::new(derived()))
    }
    pub(crate) fn new(operators: Arc<dyn OperatorsPort>, derived: Arc<dyn DerivedPort>) -> Self {
        Self {
            operators,
            derived: derived.into(),
        }
    }
    /// The program of an authored request, assembled and prepared as the engine prepares it.
    pub(crate) fn prepare(
        &self,
        request: Authored,
    ) -> Result<PreparedExecutionProgram, OperatingError> {
        let compiled = ryo_execution_domain::domain::prepared_request::service::assemble(
            request.plans,
            request.bindings,
            &request.rules,
        )?;
        self.derived.program(
            compiled,
            &request.rules,
            &request.digests,
            &request.hosted,
            request.observations,
            Arc::clone(&self.operators),
            Arc::new(ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake),
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
/// The demand of a rule's emitted root alone, its observations not required.
pub(crate) const fn emitted() -> RuleDemand {
    RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    }
}
/// Repeat the supported public owner protocol over the same immutable compiled request.
pub(crate) fn run_prepared(
    _engine: &Preparation,
    prepared: &PreparedExecutionProgram,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    facts: &mut dyn FactsWorker,
) -> Result<Table, OperatingError> {
    run_prepared_context(_engine, prepared, source, elements, symbols, facts, None)
}
/// The same public owner with its real borrowed rendering capability: file 0 run through `run_file` over `facts`, its one rule's
/// diagnostic collected. A read of the facts they refused is the engine's `NotRead`, as the engine meets it.
pub(crate) fn run_prepared_context(
    _engine: &Preparation,
    prepared: &PreparedExecutionProgram,
    source: &[u8],
    elements: &Elements,
    symbols: &mut Symbols,
    facts: &mut dyn FactsWorker,
    semantic: Option<&dyn crate::inbound::use_cases::execution::interface::SemanticLookup>,
) -> Result<Table, OperatingError> {
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(u64::try_from(source.len()).map_err(|_| OperatingError::NotRun)?),
        modified: Nanoseconds(0),
    })];
    let request = ExecutionUseCaseImpl::new().execution(prepared, &corpus, None.into());
    let rules = run_file(
        &request,
        RunFile {
            source,
            elements,
            intents: &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }],
            semantic,
        },
        symbols,
        facts,
    );
    request.finish();
    let FileRules { rules, end, .. } = rules;
    if let FileEnd::Refused(stop) = end {
        return Err(super::facts::operating(stop));
    }
    let ran = rules.into_iter().next().ok_or(OperatingError::NotRun)?;
    let table = match (ran.emit, ran.after) {
        (EmittedRoot::Delivered(RootOutcome::Complete(table)), RuleAfter::Ran { .. }) => table,
        (EmittedRoot::Delivered(RootOutcome::Failed(error)), _) => return Err(error),
        (_, RuleAfter::Stopped(stop)) => return Err(super::facts::operating(stop)),
        (EmittedRoot::Delivered(RootOutcome::Unknown(error)), _) => panic!(
            "expected a complete publication or exact Failed outcome, got Unknown({error:?})"
        ),
        (EmittedRoot::Delivered(_) | EmittedRoot::Withheld, _) => {
            panic!("authored per-file request must publish a complete table or its exact failure")
        }
    };
    Ok(table)
}
/// What file 0 of a request is run with through `run_file`: its source and tree, its rules' intents and the semantic capability.
pub(crate) struct RunFile<'a> {
    pub(crate) source: &'a [u8],
    pub(crate) elements: &'a Elements,
    pub(crate) intents: &'a [FileRuleIntent],
    pub(crate) semantic:
        Option<&'a dyn crate::inbound::use_cases::execution::interface::SemanticLookup>,
}
/// File 0 of `request` run through the one per-file route, over `facts`, every rule's first frontier bucket 0.
pub(crate) fn run_file(
    request: &RequestExecution,
    file: RunFile<'_>,
    symbols: &mut Symbols,
    facts: &mut dyn FactsWorker,
) -> FileRules {
    let first = vec![Bucket(0); file.intents.len()];
    let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
    request.run_file(
        FileRunInput {
            language: SourceLanguage::Recognized,
            file: FileId(0),
            intents: file.intents,
            source: ryo_values::domain::files::types::SourceView(file.source),
            sources: ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSources::unread(
                file.elements,
            ),
            wanted: &[],
            active: &[],
            said: &[],
            recovery: crate::inbound::use_cases::metadata::interface::Recovery::Whole,
            answering: crate::inbound::use_cases::execution::interface::Partiality::Physical,
            semantic: match file.semantic {
                Some(lookup) => SemanticSupport::Supplied(lookup),
                None => SemanticSupport::Absent,
            },
            kinds: KindSupport::Absent,
            first: &first,
        },
        symbols,
        CaptureContext {
            worker: WorkerNumber(0),
            arena: Arena(&mut arena),
            places: &mut places,
            words: &mut words,
        },
        facts,
    )
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
) -> Result<Table, OperatingError> {
    table_request_context(operation, inputs, bindings, symbols, b"", None)
}
/// Typed fixture data enters the same public owner and optional real capability.
pub(crate) fn table_request_context(
    operation: ryo_values::domain::plans::types::Operation,
    inputs: &[&Table],
    bindings: Vec<ParameterBinding>,
    symbols: &mut Symbols,
    source: &[u8],
    semantic: Option<&dyn crate::inbound::use_cases::execution::interface::SemanticLookup>,
) -> Result<Table, OperatingError> {
    let authored = table_inputs(operation, inputs, &bindings, symbols);
    let mut facts = TableFacts::new(
        &authored
            .facts
            .iter()
            .map(|(relation, table)| (*relation, table))
            .collect::<Vec<_>>(),
    );
    let (engine, prepared) = prepared(authored.plan, bindings, authored.hosted)?;
    run_prepared_context(
        &engine,
        &prepared,
        source,
        &Elements::default(),
        symbols,
        &mut facts,
        semantic,
    )
}
