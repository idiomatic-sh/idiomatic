//! One request of two written rules whose second has an authoring error (a step beyond its operator contract, or a step that
//! reads a step that has not run) answers the second rule's error through the public file owner while the first rule delivers
//! its rows: compilation and preparation succeed, and the error is the second rule's alone. Each rule's written text is
//! compiled through the query-language port and its parameters bound from its written spec; the provider hands over the
//! plans (RyoQL refuses both authoring errors itself, so a provider is the author that can hand them over).
use super::super::super::{fixtures, interface::*};
use crate::outbound::query_language::{
    errors::QueryLanguageError, fixtures::fake::QueryLanguageFake, interface::QueryLanguagePort,
};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{
        CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as CellValue,
    },
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
use ryo_values::domain::query_tokens::types::{
    QueryBundle, QueryDocumentSource, QuerySourceId, QuerySourceText,
};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};

/// Which authoring error the second rule carries.
#[derive(Clone, Copy)]
enum Authored {
    /// A step after the emit step names a column the select does not keep.
    InvalidColumn,
    /// A step reads a step that comes after it, so it reads a step that has not run.
    DanglingStep,
}
type Input = Authored;
type Value = Siblings;
/// What the two rules answered side by side.
#[derive(Debug, PartialEq, Eq)]
struct Siblings {
    /// The sound rule's row count, `None` when its root did not complete.
    sound: Option<usize>,
    /// The errored rule's error, as an `Err` of its answer or a failed root; `None` when it delivered anything else.
    errored: Option<OperatingError>,
}
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

fn an_errored_rule_answers_its_error_while_its_sibling_delivers_rows(
    authored: Authored,
) -> Result<Siblings, OperatingError> {
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
    // An errored rule does not refuse the request: it is refused on its own roots.
    let (_engine, prepared) = fixtures::requests::prepared_many(plans, bindings, vec![])?;
    let population = sources();
    let mut facts = fixtures::TableFacts::new(&[(Relation::Function, &population)]);
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("errored-rule.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::requests::execution().execution(&prepared, &corpus, None.into());
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let run = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    };
    let intents = [run, run];
    let rules = fixtures::requests::run_file(
        &request,
        fixtures::requests::RunFile {
            source: b"",
            elements: &elements,
            intents: &intents,
            semantic: None,
        },
        &mut symbols,
        &mut facts,
    )
    .rules;
    request.finish();
    let mut ran = rules.into_iter();
    let sound = match ran.next().map(|rule| rule.emit) {
        Some(EmittedRoot::Delivered(RootOutcome::Complete(rows))) => Some(rows.count.0),
        _ => None,
    };
    let errored = ran.next().and_then(|rule| match (rule.emit, rule.after) {
        (EmittedRoot::Delivered(RootOutcome::Failed(error)), _) => Some(error),
        (_, RuleAfter::Stopped(stop)) => Some(fixtures::facts::operating(stop)),
        _ => None,
    });
    Ok(Siblings { sound, errored })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_errored_rule_answers_its_error_while_its_sibling_delivers_rows"),
        word: Word::Succeeds,
        input: || Authored::InvalidColumn,
        expect: Expect::Value(Siblings {
            sound: Some(2),
            errored: Some(OperatingError::Table(RowsError::NoSuchColumn { index: 99 })),
        }),
    },
    Case {
        name: CaseName(
            "a_rule_whose_step_reads_a_step_that_has_not_run_answers_its_error_while_its_sibling_delivers_rows",
        ),
        word: Word::Succeeds,
        input: || Authored::DanglingStep,
        expect: Expect::Value(Siblings {
            sound: Some(2),
            errored: Some(OperatingError::NotRun),
        }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    an_errored_rule_answers_its_error_while_its_sibling_delivers_rows(input)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
