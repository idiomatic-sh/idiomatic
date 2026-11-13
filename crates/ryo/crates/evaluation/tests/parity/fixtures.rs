//! The two evaluations of one request on the same files: the interpreter's `run_file` over facts walked eagerly, as the parse step
//! walks them, and the fused `evaluate` over the lazy facts port.

use ryo_evaluation::inbound::use_cases::evaluate::interface::EvaluateUseCase as _;
use ryo_evaluation::inbound::use_cases::evaluate::types::EvaluateInput;
use ryo_evaluation::inbound::use_cases::evaluate::use_case::EvaluateUseCaseImpl;
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase as _;
use ryo_execution::inbound::use_cases::execution::interface::{
    Arena, CaptureContext, CaptureIntent, DiagnosticDelivery, EmittedRoot, ExecutionUseCase as _,
    FileRuleIntent, FileRunInput, KindSupport, ObservationDemand, ProgramPreparedExecutionPort as _,
    ProgramRequestExecutionPort as _, RootOutcome, SemanticSupport, SourceLanguage,
};
use ryo_execution::inbound::use_cases::prepare::interface::{ObservationIdentity, PrepareUseCase as _};
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::inbound::use_cases::parse::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest};
use ryo_relations::domain::relations::types::{FileId, Relation, Value, WorkerNumber, Words};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_values::domain::stored_facts::types::{Bucket, Digest};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The Ruff benchmark's directory: its rules and fragments.
pub(crate) fn ruff() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../languages/python/benchmarks/linters/ruff")
}

/// The Python language, as the Ruff benchmark configures it.
pub(crate) fn python() -> Arc<dyn ParseEngine> {
    ryo_python::linked_with_settings(ryo_python::settings::PythonParserSettings::new(
        ryo_python::settings::PythonTargetVersion::Py310,
    ))
    .parse_engine()
}

/// The names of the language's kinds, by number.
pub(crate) fn kind_names(parse: &dyn ParseEngine) -> Vec<String> {
    parse
        .kind_names()
        .0
        .into_iter()
        .map(|name| name.0.to_owned())
        .collect()
}

/// The rules of `codes`, with every fragment of the benchmark as a module, compiled.
pub(crate) fn compile(parse: &dyn ParseEngine, codes: &[&str]) -> (Vec<RuleSpec>, Vec<Plan>, ryo_execution::inbound::use_cases::compile_rules::interface::CompiledRules) {
    let vocabulary = ryo_values::domain::descriptions::types::KindVocabularies(vec![
        ryo_values::domain::descriptions::types::KindVocabulary {
            language: ryo_values::domain::descriptions::types::LanguageName("python".into()),
            kinds: kind_names(parse)
                .into_iter()
                .map(|name| ryo_values::domain::descriptions::types::KindWord(name.into()))
                .collect(),
        },
    ]);
    let mut modules = BTreeMap::new();
    for entry in std::fs::read_dir(ruff().join("fragments")).expect("the fragments") {
        let path = entry.expect("a fragment").path();
        if path.extension().is_some_and(|extension| extension == "ryoql") {
            let name = path.file_stem().and_then(|stem| stem.to_str()).expect("a name");
            let id = QuerySourceId(format!("python/ruff/{name}").into());
            drop(modules.insert(
                id.clone(),
                QueryDocumentSource {
                    id,
                    text: std::fs::read_to_string(&path).expect("its text").into(),
                },
            ));
        }
    }
    let modules = Arc::new(modules);
    let rules: Vec<RuleSpec> = codes
        .iter()
        .map(|code| RuleSpec {
            id: RuleId((*code).into()),
            query: QueryBundle {
                root: QueryDocumentSource {
                    id: QuerySourceId((*code).into()),
                    text: std::fs::read_to_string(ruff().join("rules").join(code).join("query.ryoql"))
                        .expect("the rule's query")
                        .into(),
                },
                modules: Arc::clone(&modules),
            },
            applicability: RuleApplicability::Always,
            given: vec![],
            observations: vec![],
            spelled: RuleParameters("".into()),
            include: vec![],
            exclude: vec![],
        })
        .collect();
    let compile = Arc::new(
        ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
            Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
            Arc::new(ryo_query_language::outbound::catalog::factory::language()),
        ),
    );
    let typecheck = Arc::new(
        ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
            Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
            Arc::new(ryo_query_language::outbound::catalog::factory::language()),
        ),
    );
    let compiler = ryo_execution::inbound::use_cases::compile_rules::use_case::CompileRulesUseCaseImpl::new(
        Arc::new(ryo_execution::outbound::query_language::factory::use_case(compile, typecheck)),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    let compiled = compiler
        .compile_rules(ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest {
            rules: &rules,
            hosted: &[],
            vocabularies: &vocabulary,
        })
        .expect("the rules compile");
    let plans = compiled.plans().to_vec();
    (rules, plans, compiled)
}

/// One parsed file: its tree and its names, as the parse step's `build` leaves them.
pub(crate) fn parse(parse: &dyn ParseEngine, path: &str, bytes: &[u8], file: FileId) -> (Elements, Symbols) {
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    ryo_language::inbound::use_cases::parse::service::build(
        parse,
        ParseRequest {
            path: ryo_language::domain::files::types::SourcePath(path),
            file,
            bytes: ByteSlice(bytes),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .expect("the file parses");
    ryo_language::domain::kind_index::service::index_kinds(&mut elements);
    (elements, symbols)
}

/// The relations the plans read, besides the parser's tree.
pub(crate) fn wanted(plans: &[Plan]) -> Vec<Relation> {
    let mut wanted = Vec::new();
    for plan in plans {
        for step in &plan.steps {
            let text = format!("{:?}", step.operation);
            for relation in ryo_relations::domain::relations::service::all() {
                if matches!(relation, Relation::SyntaxTree | Relation::SyntaxChild) || wanted.contains(relation) {
                    continue;
                }
                if text.contains(&format!("{:?}", Input::Relation(*relation))) {
                    wanted.push(*relation);
                }
            }
        }
    }
    wanted
}

/// One rule's emission for a file: the rule, and its emitted rows, each rendered with its names' text, in emitted order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Emission {
    /// The rule's place.
    pub rule: usize,
    /// The emitted rows, sorted, or what the outcome was when it delivered none.
    pub rows: Result<Vec<Vec<String>>, String>,
    /// Where each emitted row is: the start and end of its first span, sorted.
    pub places: Vec<(u32, u32)>,
}

/// A value as text: a name by its text, anything else as it is.
fn rendered(value: Value, symbols: &Symbols, source: &[u8]) -> String {
    match value {
        Value::Symbol(symbol) => format!(
            "text:{:?}",
            ryo_language::domain::symbols::service::text_of(symbols, ByteSlice(source), symbol)
                .map(|text| text.0.to_owned())
        ),
        other @ (Value::Node(_)
        | Value::Name(_)
        | Value::File(_)
        | Value::Span(_)
        | Value::Number(_)
        | Value::Flag(_)) => format!("{other:?}"),
    }
}

/// The rows of `table`, rendered.
fn rows_of(table: &Table, symbols: &Symbols, source: &[u8]) -> Vec<Vec<String>> {
    let width = table.cells.len().checked_div(table.count.0).unwrap_or(0);
    let mut rows: Vec<Vec<String>> = (0..table.count.0)
        .map(|at| {
            table.cells[at * width..(at + 1) * width]
                .iter()
                .map(|value| rendered(*value, symbols, source))
                .collect()
        })
        .collect();
    rows.sort();
    rows
}

/// The first span of each row of `table`, sorted.
fn places_of(table: &Table) -> Vec<(u32, u32)> {
    let width = table.cells.len().checked_div(table.count.0).unwrap_or(0);
    let mut places: Vec<(u32, u32)> = (0..table.count.0)
        .filter_map(|at| {
            table.cells[at * width..(at + 1) * width].iter().find_map(|value| match value {
                Value::Span(span) => Some((span.start.0, span.end.0)),
                _ => None,
            })
        })
        .collect();
    places.sort_unstable();
    places
}

/// What a file's run delivered, rule by rule in the order they ran.
fn emissions(ran: &[ryo_execution::inbound::use_cases::execution::interface::RanRule], symbols: &Symbols, source: &[u8]) -> Vec<Emission> {
    ran.iter()
        .map(|each| Emission {
            rule: each.rule.0,
            places: match &each.emit {
                EmittedRoot::Delivered(RootOutcome::Complete(table)) => places_of(table),
                _ => Vec::new(),
            },
            rows: match &each.emit {
                EmittedRoot::Delivered(RootOutcome::Complete(table)) => Ok(rows_of(table, symbols, source)),
                EmittedRoot::Delivered(RootOutcome::Partial { table, .. }) => {
                    Err(format!("partial {:?}", rows_of(table, symbols, source)))
                }
                EmittedRoot::Delivered(RootOutcome::Failed(error)) => Err(format!("failed {error}")),
                EmittedRoot::Delivered(RootOutcome::Unknown(error)) => Err(format!("unknown {error}")),
                EmittedRoot::Delivered(_) => Err("other".to_owned()),
                EmittedRoot::Withheld => Err("withheld".to_owned()),
            },
        })
        .collect()
}

/// The intents of a file that runs every one of `count` rules with its diagnostic collected.
fn intents(count: usize) -> Vec<FileRuleIntent> {
    (0..count)
        .map(|_| FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::DiagnosticAndObservations,
        })
        .collect()
}

/// The program the interpreter runs `rules` with.
pub(crate) fn prepared(
    rules: &[RuleSpec],
    compiled: ryo_execution::inbound::use_cases::compile_rules::interface::CompiledRules,
) -> ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram {
    let prepare = ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl::new(
        Arc::new(ryo_execution_runtime::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            )),
        )),
        ryo_execution::derived(),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    prepare
        .prepare(PrepareRequest {
            plans: PlanSource::Compiled(compiled),
            rules: rules.to_vec(),
            digests: (0..rules.len())
                .map(|place| Digest([u8::try_from(place % 256).expect("a byte"); 16]))
                .collect(),
            hosted: vec![],
            observations: ObservationIdentity::Independent,
        })
        .expect("the program prepares")
}

/// The interpreter's emissions of `codes` on each of `files`, given as path and bytes.
pub(crate) fn interpreted(codes: &[&str], files: &[(String, Vec<u8>)]) -> Vec<Vec<Emission>> {
    let language = python();
    let (rules, plans, compiled) = compile(&*language, codes);
    let wanted = wanted(&plans);
    let mut first = Vec::new();
    let mut next = Bucket(1);
    for plan in &plans {
        first.push(next);
        next = Bucket(next.0 + u32::try_from(plan.frontier.len()).expect("a count"));
    }
    let program = prepared(&rules, compiled);
    let corpus: Vec<_> = files
        .iter()
        .map(|(path, bytes)| {
            ryo_values::domain::files::types::CorpusInput::Available(ryo_values::domain::files::types::CorpusEntry {
                executable: ryo_values::domain::files::types::ExecutablePermission::Unknown,
                path: ryo_language::domain::files::types::FilePath(path.as_str().into()),
                size: ryo_values::domain::files::types::FileSize(u64::try_from(bytes.len()).expect("a size")),
                modified: ryo_values::domain::files::types::Nanoseconds(0),
            })
        })
        .collect();
    let request = ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl::new()
        .execution(&program, &corpus, None.into());
    let facts = ryo_execution_runtime::outbound::facts::factory::memory();
    let intents = intents(rules.len());
    files
        .iter()
        .enumerate()
        .map(|(number, (path, bytes))| {
            let file = FileId(u32::try_from(number).expect("a file number"));
            let (elements, mut symbols) = parse(&*language, path, bytes, file);
            let mut elements = elements;
            let mut worker = ryo_execution_runtime::outbound::facts::interface::FactsPort::worker(&facts)
                .expect("a worker");
            {
                let mut columns = worker.columns_mut().expect("columns");
                ryo_language::inbound::use_cases::parse::service::walk(
                    &*language,
                    &mut elements,
                    &mut symbols,
                    ByteSlice(bytes),
                    file,
                    &wanted,
                    &mut columns,
                );
            }
            let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
            let ran = request.run_file(
                FileRunInput {
                    language: SourceLanguage::Recognized,
                    file,
                    evaluation: &[],
                    publication: &[],
                    intents: &intents,
                    source: ryo_values::domain::files::types::SourceView(bytes),
                    elements: &elements,
                    semantic: SemanticSupport::Absent,
                    kinds: KindSupport::Absent,
                    first: &first,
                },
                &mut symbols,
                CaptureContext {
                    worker: WorkerNumber(0),
                    arena: Arena(&mut arena),
                    places: &mut places,
                    words: &mut words,
                },
                &*worker,
            );
            emissions(&ran.rules, &symbols, bytes)
        })
        .collect()
}

/// The fused evaluator's emissions of `codes` on each of `files`, with the relations each file asked the facts port for.
pub(crate) fn fused(codes: &[&str], files: &[(String, Vec<u8>)]) -> Vec<(Vec<Emission>, usize, Vec<Relation>)> {
    let language = python();
    let (rules, plans, compiled) = compile(&*language, codes);
    let order = prepared(&rules, compiled).rule_order().collect();
    let names = kind_names(&*language);
    let kinds: Vec<&str> = names.iter().map(String::as_str).collect();
    let evaluate = EvaluateUseCaseImpl::new(&plans, order, &kinds).expect("the plans fuse");
    let intents = intents(plans.len());
    files
        .iter()
        .enumerate()
        .map(|(number, (path, bytes))| {
            let file = FileId(u32::try_from(number).expect("a file number"));
            let (mut elements, mut symbols) = parse(&*language, path, bytes, file);
            let facts = ryo_evaluation::outbound::facts::fixtures::CountedWalks {
                inner: ryo_evaluation::outbound::facts::factory::file(&*language, &mut elements, bytes, file),
                walks: std::cell::Cell::new(0),
                asked: std::cell::RefCell::new(Vec::new()),
            };
            let ran = evaluate.evaluate(
                EvaluateInput {
                    file,
                    source: bytes,
                    intents: &intents,
                },
                &mut symbols,
                &facts,
            );
            let walks = facts.walks.get();
            let asked = facts.asked.borrow().clone();
            (emissions(&ran.rules, &symbols, bytes), walks, asked)
        })
        .collect()
}
