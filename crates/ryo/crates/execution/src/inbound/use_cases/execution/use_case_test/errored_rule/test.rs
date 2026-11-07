//! One request of two written rules whose second has an authoring error (a step beyond its operator contract, or a step that
//! reads a step that has not run) answers the second rule's error through the public file owner while the first rule delivers
//! its rows: compilation and preparation succeed, and the error is the second rule's alone. Each rule's written text is
//! compiled through the query-language port and its parameters bound from its written spec; the provider hands over the
//! plans (RyoQL refuses both authoring errors itself, so a provider is the author that can hand them over).
use crate::application::file_dag::program::file_session::interface::FileExecution as _;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::outbound::facts::types::ResidentStability;
use crate::application::file_dag::program::file_session::types::{FileInput, SourceLanguage, SourceRows};
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use super::super::super::{fixtures, interface::*};
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use crate::outbound::query_language::{
    errors::QueryLanguageError, fixtures::fake::QueryLanguageFake, interface::QueryLanguagePort,
};
use ryo_values::domain::query_tokens::types::{
    QueryBundle, QueryDocumentSource, QuerySourceId, QuerySourceText,
};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as CellValue},
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::ExecutablePermission;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use crate::application::file_dag::program::prepared_execution::service::prepare;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::domain::prepared_request::types::ObservationIdentity;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::stored_facts::types::Digest;
use std::collections::BTreeMap;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs::text;

/// Which authoring error the second rule carries.
#[derive(Clone, Copy)]
enum Authored {
    /// A step after the emit step names a column the select does not keep.
    InvalidColumn,
    /// A step reads a step that comes after it, so it reads a step that has not run.
    DanglingStep,
}
type Input = Authored;
type Value = ();
type Failure = OperatingError;
fn sources() -> Table {
    let kinds = kinds(Relation::Function).to_vec();
    let mut cells = Vec::new();
    for row in 1..=2 {
        cells.extend(kinds.iter().map(|kind| match kind {
            CellKind::Node => CellValue::Node(ryo_relations::domain::relations::service::node_id(
                FileId(0),
                Position(row),
            )),
            CellKind::Symbol => CellValue::Symbol(Symbol(0)),
            CellKind::File => CellValue::File(FileId(0)),
            CellKind::Span => CellValue::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            CellKind::Number => CellValue::Number(Integer(0)),
            CellKind::Flag => CellValue::Flag(Flag::Lowered),
        }));
    }
    Table {
        kinds,
        cells,
        count: RowCount(2),
    }
}

/// The query-language provider the rules are written against: each written text compiles to the plan handed over for it.
struct Provider(Vec<(&'static str, QueryLanguageFake)>);

fn written<'a>(provider: &'a Provider, text: &QueryBundle) -> &'a QueryLanguageFake {
    provider
        .0
        .iter()
        .find(|(written, _)| *written == &*text.root.text)
        .map(|(_, fake)| fake)
        .expect("every written rule has a text the provider compiles")
}

impl QueryLanguagePort for Provider {
    fn validate(
        &self,
        bundle: &QueryBundle,
        hosted: &[ryo_values::domain::hosted::types::HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ryo_values::domain::descriptions::types::ValidateReport, QueryLanguageError> {
        written(self, bundle).validate(bundle, hosted, observations)
    }
    fn compile(
        &self,
        bundle: &QueryBundle,
        hosted: &[ryo_values::domain::hosted::types::HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ryo_values::domain::plans::types::Plan, QueryLanguageError> {
        written(self, bundle).compile(bundle, hosted, observations)
    }
}

/// A rule written as a user writes it: an id and its query text.
fn written_rule(id: &str, text: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(id.into()),
                text: QuerySourceText(text.into()),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    }
}

fn an_errored_rule_answers_its_error_while_its_sibling_delivers_rows(authored: Authored) {
    let select = || Operation::Select {
        from: PlanInput::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    };
    let sound = inputs::plan(vec![select()]);
    let mut errored = match authored {
        // The step after the emit step names a column that is not there: the rule's origin is errored although its emit
        // step is sound.
        Authored::InvalidColumn => inputs::plan(vec![
            select(),
            Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(99)],
                distinct: RepeatedRows::Kept,
            },
        ]),
        // The step after the emit step reads the step after it: it reads a step that has not run.
        Authored::DanglingStep => inputs::plan(vec![
            select(),
            Operation::Project {
                from: PlanInput::Step(StepId(2)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
        ]),
    };
    errored.emit.step = StepId(0);
    let rules = vec![
        written_rule("sound", "sound rule"),
        written_rule("errored", "errored rule"),
    ];
    let provider = Provider(vec![
        (
            "sound rule",
            QueryLanguageFake {
                plan: sound,
                ..QueryLanguageFake::default()
            },
        ),
        (
            "errored rule",
            QueryLanguageFake {
                plan: errored,
                ..QueryLanguageFake::default()
            },
        ),
    ]);
    let plans = rules
        .iter()
        .map(|rule| provider.compile(&rule.query, &[], &[]))
        .collect::<Result<Vec<_>, _>>()
        .expect("both written rules compile");
    let bindings = plans
        .iter()
        .zip(&rules)
        .map(|(plan, rule)| ryo_values::domain::rule_requests::service::bind(plan, rule))
        .collect::<Result<Vec<_>, _>>()
        .expect("both written rules bind their parameters");
    let (engine, prepared) = fixtures::prepared_many(plans, bindings, vec![])
        .expect("an errored rule does not refuse the request: it is refused on its own roots");
    let population = sources();
    let read = |relation| match relation {
        Relation::Function => Ok(SourceRows::Computed(&population)),
        _ => Err(OperatingError::NotRead),
    };
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("errored-rule.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::execution().execution(&prepared, &corpus);
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let run = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    };
    let intents = [run, run];
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &intents,
                source: ryo_values::domain::files::types::SourceView(b""),
                elements: &elements,
                read: &read,
                semantic: crate::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: crate::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("the file owner opens over both rules");
    let demand = || RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let sibling = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("the sound rule answers");
    match sibling.emit {
        RootOutcome::Complete(rows) => assert_eq!(rows.count, RowCount(2), "the sound rule's rows"),
        _ => panic!("the sound rule delivers its rows beside an errored sibling"),
    }
    let refused = file.rule(RuleIndex(1), demand(), &mut symbols);
    let delivered = match refused {
        Err(error) => Some(error),
        Ok(answer) => match answer.emit {
            RootOutcome::Failed(error) => Some(error),
            _ => None,
        },
    };
    assert!(
        delivered.is_some(),
        "the errored rule reports its error and never delivers rows"
    );
    file.finish().expect("the file owner finishes");
    request.finish();
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_errored_rule_answers_its_error_while_its_sibling_delivers_rows"),
        word: Word::Succeeds,
        input: || Authored::InvalidColumn,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_rule_whose_step_reads_a_step_that_has_not_run_answers_its_error_while_its_sibling_delivers_rows"),
        word: Word::Succeeds,
        input: || Authored::DanglingStep,
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    an_errored_rule_answers_its_error_while_its_sibling_delivers_rows(input);
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
/// One request of several authored rules, the first named as `prepared` names its only one.
fn prepared_many(
    plans: Vec<Plan>,
    bindings: Vec<Vec<ParameterBinding>>,
    hosted: Vec<HostRelation>,
) -> Result<(fixtures::Preparation, PreparedExecutionProgram), OperatingError> {
    let engine = fixtures::Preparation::real();
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
    let prepared = engine.prepare(fixtures::Authored { plans, bindings, rules, digests, hosted, observations: ObservationIdentity::Independent })?;
    Ok((engine, prepared))
}
