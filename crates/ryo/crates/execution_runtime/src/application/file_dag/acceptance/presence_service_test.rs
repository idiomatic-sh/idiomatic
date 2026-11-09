// ignore: idiomatic.arch/file-length - the R1 contract cases of one design share one fixture program, one file harness and one probe; splitting would duplicate the harness.
//! A rule that cannot match a file is never instantiated there: its root publishes the empty, complete table its full run
//! would, no region only it reaches gets a slot or a reader count, and nothing that could have a row or a row error is
//! skipped. Language-free: fixture relations, fixture rows and compiled programs only.
use super::constants::NO_TEXT;
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::{FileRun, open_file};
use crate::application::file_dag::program::facade_runtime::types::{
    InputRelease, Instantiated, ProgramServices, RegionProbe,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::{Partial, RuleIndex};
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::observation_schedule::types::RuleSchedule;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use ryo_execution_domain::domain::rule_intent::types::{
    ApplicabilityReason, CaptureIntent, DiagnosticDelivery, EmitDemand, FileRuleIntent,
    ObservationDemand, RuleDemand,
};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Flag, NodeId, Relation, Span, Value,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds, SourceView,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, JoinColumn, LiteralText, Operand, PlanComparison,
    QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;

/// The fixture file's source text: no name reads it, every text is derived.
const SOURCE: &[u8] = b"";

/// The text column of both fixture relations: `comment.text` and `function.name`.
const TEXT: u32 = 1;

fn equal(column: u32, text: &str) -> PlanComparison {
    compare(column, Test::Equal, text)
}
fn compare(column: u32, test: Test, text: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}
fn select(from: Input, comparisons: Vec<PlanComparison>) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons,
        kept: vec![],
    })
}
fn at(step: u32) -> Input {
    Input::Step(StepId(step))
}
/// A join of two steps on the file column, keeping the left columns.
fn join_on_file(left: u32, right: u32, width: u32) -> PlanStep {
    step(Operation::Join {
        left: at(left),
        right: at(right),
        left_on: vec![ColumnIndex(2)],
        right_on: vec![ColumnIndex(2)],
        output: (0..width)
            .map(|column| JoinColumn {
                side: Side::Left,
                column: ColumnIndex(column),
            })
            .collect(),
        filter: vec![],
        view: vec![],
        reach: None,
    })
}
fn plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(u32::try_from(steps.len() - 1).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
    }
}
/// Rule B: comments whose text is "y", in a file with a function. It cannot match a file whose comments say "x".
fn rule_b() -> Plan {
    plan(vec![
        select(Input::Relation(Relation::Comment), vec![equal(TEXT, "y")]),
        select(Input::Relation(Relation::Function), vec![]),
        join_on_file(0, 1, 5),
    ])
}
fn spec(at: usize) -> RuleSpec {
    let id = format!("presence-{at}");
    RuleSpec {
        id: RuleId(id.clone().into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(id.into()),
                text: NO_TEXT.into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters(NO_TEXT.into()),
        include: vec![],
        exclude: vec![],
    }
}
fn prepare(plans: Vec<Plan>) -> Arc<PreparedProgram> {
    let count = plans.len();
    let specs: Vec<RuleSpec> = (0..count).map(spec).collect();
    crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; count],
            plans,
            bindings: vec![vec![]; count],
        },
        &specs,
        &vec![Digest([0; 16]); count],
        &[],
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
        ObservationIdentity::Independent,
    )
    .expect("the fixture program prepares")
}
fn services() -> Arc<ProgramServices> {
    Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("presence".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ))
}

/// One fixture file: its comments' and functions' texts, interned as derived names.
struct Facts {
    symbols: Symbols,
    comment: Table,
    function: Table,
}
fn span(at: usize) -> Value {
    Value::Span(Span {
        start: ryo_relations::domain::relations::types::Offset(u32::try_from(at).unwrap_or(0)),
        end: ryo_relations::domain::relations::types::Offset(u32::try_from(at).unwrap_or(0)),
    })
}
fn facts(comments: &[&str], functions: &[&str]) -> Facts {
    let mut symbols = Symbols::default();
    let mut name = |text: &str| {
        Value::Symbol(ryo_language::domain::symbols::service::intern_derived(
            &mut symbols,
            ByteSlice(SOURCE),
            NameText(text),
        ))
    };
    let comment = table(
        Relation::Comment,
        comments
            .iter()
            .enumerate()
            .map(|(at, text)| {
                vec![
                    Value::Node(NodeId(at as u64)),
                    name(text),
                    Value::File(FileId(0)),
                    span(at),
                    Value::Flag(Flag::Lowered),
                ]
            })
            .collect(),
    );
    let function = table(
        Relation::Function,
        functions
            .iter()
            .enumerate()
            .map(|(at, text)| {
                vec![
                    Value::Node(NodeId(100 + at as u64)),
                    name(text),
                    Value::File(FileId(0)),
                    span(at),
                ]
            })
            .collect(),
    );
    Facts {
        symbols,
        comment,
        function,
    }
}
fn table(relation: Relation, rows: Vec<Vec<Value>>) -> Table {
    Table {
        kinds: kinds(relation).to_vec(),
        count: RowCount(rows.len()),
        cells: rows.into_iter().flatten().collect(),
    }
}
fn run_intent() -> FileRuleIntent {
    FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }
}

/// What one file run answered and left behind.
struct Ran {
    outcomes: Vec<Result<RootOutcome, OperatingError>>,
    fed: Vec<Vec<u64>>,
    probes: Vec<RegionProbe>,
    presence_absent: u64,
    again: Vec<Result<(), OperatingError>>,
    cleanup: Result<(), OperatingError>,
    dormant: usize,
    finished: Result<(), OperatingError>,
}
/// Runs every rule of `prepared` on one file in rule order, then probes `regions` and finishes the file.
fn run(
    prepared: &Arc<PreparedProgram>,
    facts: &mut Facts,
    intents: &[FileRuleIntent],
    partial: &[Partial],
    regions: &[RegionId],
) -> Ran {
    let Facts {
        symbols,
        comment,
        function,
    } = facts;
    let location = table(Relation::SourceLocation, vec![]);
    let read = |relation| match relation {
        Relation::Comment => Ok(Rows::Computed(&*comment)),
        Relation::Function => Ok(Rows::Computed(&*function)),
        Relation::SourceLocation => Ok(Rows::Computed(&location)),
        _ => Err(OperatingError::NotRead),
    };
    let mut file: FileRun<'_> = open_file(
        prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: partial,
            publication: &[],
        },
        intents,
        services(),
        SourceView(SOURCE),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        symbols,
        None,
        None,
    )
    .expect("the file opens");
    let mut outcomes = Vec::new();
    let mut fed = Vec::new();
    for (rule, intent) in intents.iter().enumerate() {
        if !matches!(intent, FileRuleIntent::Run { .. }) {
            outcomes.push(Err(OperatingError::NotRun));
            fed.push(Vec::new());
            continue;
        }
        let outcome = file.rule(
            RuleIndex(rule),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        );
        fed.push(
            file.counts(
                RuleIndex(rule),
                crate::application::file_dag::program::runtime::types::FedRead::Collect,
            )
                .map(|counts| counts.fed.iter().map(|each| each.0.iter().sum()).collect())
                .unwrap_or_default(),
        );
        outcomes.push(outcome.map(|outcome| outcome.emit));
    }
    let probes = regions
        .iter()
        .map(|region| file.region_probe(*region))
        .collect();
    let presence_absent = file.readiness_counts().presence_absent.0;
    let cleanup = file.cleanup_status();
    let again = regions
        .iter()
        .map(|region| file.retire_again(*region))
        .collect();
    let dormant = file.state_counts().dormant_obligations.0;
    let finished = file.finish().map(|_| ());
    Ran {
        outcomes,
        fed,
        probes,
        presence_absent,
        again,
        cleanup,
        dormant,
        finished,
    }
}
/// The region each step of a rule runs in.
fn regions_of(prepared: &PreparedProgram, rule: usize) -> Vec<RegionId> {
    let program = prepared.program();
    program.rules()[rule]
        .original_steps
        .iter()
        .map(|origin| {
            let origin = origin.as_ref().expect("every step is placed");
            program.logical_regions()[origin.logical.0]
        })
        .collect()
}
fn rows(outcome: &Result<RootOutcome, OperatingError>) -> Option<(usize, Vec<CellKind>)> {
    match outcome {
        Ok(RootOutcome::Complete(table)) => Some((table.count.0, table.kinds.clone())),
        _ => None,
    }
}
fn error(outcome: &Result<RootOutcome, OperatingError>) -> Option<OperatingError> {
    match outcome {
        Ok(RootOutcome::Failed(error) | RootOutcome::Unknown(error)) | Err(error) => {
            Some(error.clone())
        }
        _ => None,
    }
}

/// R1.1 and R1.2: B is absent. Its root has one slot, published and `Consumed`; no dependency of B has a slot or a reader
/// count; cleanup records nothing. Its answer equals the full run's on a control file where B's formula holds but its join
/// drops every row, and equals an antijoin's empty-input gate over the same shape there.
#[test]
fn an_absent_rule_is_never_instantiated_and_answers_as_its_full_run() {
    let gate = plan(vec![
        select(Input::Relation(Relation::Comment), vec![equal(TEXT, "y")]),
        select(Input::Relation(Relation::Function), vec![]),
        join_on_file(0, 1, 5),
        select(Input::Relation(Relation::Comment), vec![]),
        step(Operation::Antijoin {
            left: at(2),
            right: at(3),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        }),
    ]);
    let prepared = prepare(vec![rule_b(), gate]);
    let b = regions_of(&prepared, 0);
    let intents = [run_intent(), run_intent()];
    let absent = run(&prepared, &mut facts(&["x"], &["f"]), &intents, &[], &b);
    let (root, dependencies) = b.split_last().expect("B has steps");
    assert!(
        dependencies.iter().all(|region| region != root),
        "B's root is its own region"
    );
    let root_probe = &absent.probes[b.len() - 1];
    assert_eq!(
        root_probe.instantiated,
        Instantiated::Slot,
        "B's root has one slot"
    );
    assert_eq!(root_probe.inputs, Some(InputRelease::Released));
    for (region, probe) in dependencies.iter().zip(&absent.probes) {
        assert_eq!(
            probe.instantiated,
            Instantiated::Never,
            "B's input region {region:?} was never instantiated"
        );
    }
    assert_eq!(absent.cleanup, Ok(()));
    assert_eq!(absent.finished, Ok(()));
    assert!(absent.presence_absent >= 1);
    // The control file: a "y" comment, so B's formula holds, but no function, so the join drops every row.
    let control = run(&prepared, &mut facts(&["y"], &[]), &intents, &[], &b);
    let empty = Some((0, kinds(Relation::Comment).to_vec()));
    assert_eq!(rows(&absent.outcomes[0]), empty);
    assert_eq!(rows(&control.outcomes[0]), empty);
    assert_eq!(
        rows(&control.outcomes[1]),
        empty,
        "the antijoin gate publishes the same shape"
    );
    // Negative control: on the control file B's root ran in full and released its inputs; a second release still refuses.
    assert_eq!(control.again[b.len() - 1], Err(OperatingError::NotRun));
}

/// R1.3: rules whose answer is an absence or an alternative are not skipped where they have rows.
#[test]
fn absences_and_alternatives_are_never_skipped_where_they_have_rows() {
    let antijoin = plan(vec![
        select(Input::Relation(Relation::Comment), vec![equal(TEXT, "x")]),
        select(Input::Relation(Relation::Function), vec![equal(TEXT, "y")]),
        step(Operation::Antijoin {
            left: at(0),
            right: at(1),
            left_on: vec![ColumnIndex(2)],
            right_on: vec![ColumnIndex(2)],
        }),
    ]);
    let union = plan(vec![
        select(Input::Relation(Relation::Comment), vec![equal(TEXT, "y")]),
        select(Input::Relation(Relation::Comment), vec![equal(TEXT, "x")]),
        step(Operation::Union {
            left: at(0),
            right: at(1),
        }),
    ]);
    let prepared = prepare(vec![antijoin, union]);
    let ran = run(
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent(), run_intent()],
        &[],
        &[],
    );
    assert_eq!(rows(&ran.outcomes[0]).map(|(count, _)| count), Some(1));
    assert_eq!(rows(&ran.outcomes[1]).map(|(count, _)| count), Some(1));
    assert_eq!(ran.finished, Ok(()));
}

/// R1.4: `l.text = "x" and r.text = "z"` over a self join, on a file with x and z in different rows, runs and answers.
#[test]
fn a_self_join_with_a_literal_on_each_side_runs() {
    let self_join = plan(vec![
        select(Input::Relation(Relation::Comment), vec![]),
        step(Operation::Join {
            left: at(0),
            right: at(0),
            left_on: vec![ColumnIndex(2)],
            right_on: vec![ColumnIndex(2)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(TEXT),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(TEXT),
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
        select(at(1), vec![equal(0, "x"), equal(1, "z")]),
    ]);
    let prepared = prepare(vec![self_join]);
    let ran = run(
        &prepared,
        &mut facts(&["x", "z"], &[]),
        &[run_intent()],
        &[],
        &[],
    );
    assert_eq!(rows(&ran.outcomes[0]).map(|(count, _)| count), Some(1));
    assert_eq!(ran.presence_absent, 0);
}

/// R1.5: a join with no view runs where both sides have rows and is absent where its right side has none. How a filter,
/// a view and a reach join resolve their right columns is pinned by the fold's own cases.
#[test]
fn a_join_without_a_view_runs_where_both_sides_have_rows() {
    let joined = plan(vec![
        select(Input::Relation(Relation::Comment), vec![]),
        select(Input::Relation(Relation::Function), vec![equal(TEXT, "f")]),
        step(Operation::Join {
            left: at(0),
            right: at(1),
            left_on: vec![ColumnIndex(2)],
            right_on: vec![ColumnIndex(2)],
            output: vec![JoinColumn {
                side: Side::Right,
                column: ColumnIndex(TEXT),
            }],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
    ]);
    let prepared = prepare(vec![joined]);
    let ran = run(
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        &[],
        &[],
    );
    assert_eq!(
        rows(&ran.outcomes[0]).map(|(count, _)| count),
        Some(1),
        "{:?}",
        error(&ran.outcomes[0])
    );
    assert_eq!(ran.presence_absent, 0);
    let skipped = run(
        &prepared,
        &mut facts(&["x"], &["g"]),
        &[run_intent()],
        &[],
        &[],
    );
    assert_eq!(rows(&skipped.outcomes[0]).map(|(count, _)| count), Some(0));
    assert!(skipped.presence_absent >= 1);
}

/// R1.6: a row-fallible operator upstream keeps the rule running where its input has rows, so its row error is reported
/// as in the full run; Replace over a Symbol column is no such operator, Replace over a Node column is.
#[test]
fn row_errors_upstream_keep_the_rule_running() {
    let absent = || select(Input::Relation(Relation::Function), vec![equal(TEXT, "y")]);
    let ordered = plan(vec![
        select(Input::Relation(Relation::Comment), vec![]),
        step(Operation::Extend {
            from: at(0),
            values: vec![Operand::Literal(QueryLiteral::Number(
                ryo_relations::domain::relations::types::Integer(1),
            ))],
        }),
        absent(),
        step(Operation::Extend {
            from: at(2),
            values: vec![Operand::Literal(QueryLiteral::Number(
                ryo_relations::domain::relations::types::Integer(1),
            ))],
        }),
        step(Operation::OrderedMatch {
            left: at(1),
            right: at(3),
            left_on: vec![],
            right_on: vec![],
            left_order: ColumnIndex(5),
            right_order: ColumnIndex(4),
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            }],
        }),
    ]);
    let bounds = plan(vec![
        select(Input::Relation(Relation::Comment), vec![]),
        step(Operation::SpanBounds {
            from: at(0),
            span: ColumnIndex(3),
        }),
        absent(),
        join_on_file(1, 2, 1),
    ]);
    let replace = |column: u32| {
        plan(vec![
            select(Input::Relation(Relation::Comment), vec![]),
            step(Operation::Replace {
                from: at(0),
                text: ColumnIndex(column),
                pattern: LiteralText("a".into()),
                replacement: LiteralText("b".into()),
            }),
            absent(),
            join_on_file(1, 2, 1),
        ])
    };
    let prepared = prepare(vec![ordered, bounds, replace(TEXT), replace(0)]);
    let mut file = facts(&["x", "x"], &["f"]);
    if let Some(cell) = file.comment.cells.get_mut(3) {
        *cell = Value::Span(ryo_relations::domain::relations::constants::NO_SPAN);
    }
    let intents = [run_intent(), run_intent(), run_intent(), run_intent()];
    let symbol = regions_of(&prepared, 2);
    let ran = run(&prepared, &mut file, &intents, &[], &symbol);
    assert_eq!(
        error(&ran.outcomes[0]),
        Some(OperatingError::DuplicateOrder)
    );
    assert!(matches!(
        error(&ran.outcomes[1]),
        Some(OperatingError::InvalidSpanBounds { .. })
    ));
    assert_eq!(rows(&ran.outcomes[2]).map(|(count, _)| count), Some(0));
    assert_eq!(symbol.len(), 4);
    assert_eq!(
        ran.probes[1].instantiated,
        Instantiated::Never,
        "the Symbol Replace never ran"
    );
    assert_eq!(error(&ran.outcomes[3]), Some(OperatingError::ShapeMismatch));
}

/// R1.6: a literal pattern that does not compile is refused when a row first reaches it. A Replace over a Symbol column
/// cannot fail on its rows, but with such a pattern its input is fallible, so the rule runs where the input has rows and
/// reports the refusal as its full run does, even where its other conjunct cannot match.
#[test]
fn an_invalid_replace_pattern_is_reported_where_its_input_has_rows() {
    let invalid = plan(vec![
        select(Input::Relation(Relation::Comment), vec![]),
        step(Operation::Replace {
            from: at(0),
            text: ColumnIndex(TEXT),
            pattern: LiteralText("(".into()),
            replacement: LiteralText("b".into()),
        }),
        select(Input::Relation(Relation::Function), vec![equal(TEXT, "y")]),
        join_on_file(1, 2, 1),
    ]);
    let prepared = prepare(vec![invalid]);
    let ran = run(
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        &[],
        &[],
    );
    let reported = error(&ran.outcomes[0]);
    assert!(
        matches!(reported, Some(OperatingError::InvalidRegex { .. })),
        "{reported:?}"
    );
}

/// R1.6: a select's literal `matches` pattern is compiled when a row first reaches it, so one that does not compile, by
/// its syntax or by the compiled size limit, makes the select's input fallible: the rule runs where that input has rows
/// and reports the refusal as its full run does, even where its other conjunct cannot match.
#[test]
fn an_invalid_select_pattern_is_reported_where_its_input_has_rows() {
    for pattern in ["(", r"\w{1000}{1000}"] {
        let invalid = plan(vec![
            select(Input::Relation(Relation::Comment), vec![]),
            select(at(0), vec![compare(TEXT, Test::Matches, pattern)]),
            select(Input::Relation(Relation::Function), vec![equal(TEXT, "y")]),
            join_on_file(1, 2, 1),
        ]);
        let prepared = prepare(vec![invalid]);
        let ran = run(
            &prepared,
            &mut facts(&["x"], &["f"]),
            &[run_intent()],
            &[],
            &[],
        );
        let reported = error(&ran.outcomes[0]);
        assert!(
            matches!(reported, Some(OperatingError::InvalidRegex { .. })),
            "{pattern}: {reported:?}"
        );
    }
}

/// R1.7: rule A's absent root reads X, which reads Y; A is also an input of admitted rule F. X and Y are never visited and
/// nothing is left owed at the end; when F also reads Y, Y runs once, for F.
#[test]
fn an_absent_root_inside_an_admitted_rule_leaves_nothing_owed() {
    // Y, X (a self join, its own region), the absent conjunct, and A's root joining X to it.
    let a = || {
        vec![
            select(Input::Relation(Relation::Comment), vec![]),
            join_on_file(0, 0, 5),
            select(Input::Relation(Relation::Function), vec![equal(TEXT, "y")]),
            join_on_file(1, 2, 5),
        ]
    };
    for shared in [false, true] {
        let mut f = a();
        f.push(select(
            Input::Relation(Relation::Comment),
            vec![equal(TEXT, "x")],
        ));
        f.push(step(Operation::Union {
            left: at(3),
            right: at(if shared { 0 } else { 4 }),
        }));
        let prepared = prepare(vec![plan(a()), plan(f)]);
        let regions = regions_of(&prepared, 0);
        let intents = [run_intent(), run_intent()];
        let ran = run(
            &prepared,
            &mut facts(&["x"], &["f"]),
            &intents,
            &[],
            &regions,
        );
        assert_eq!(rows(&ran.outcomes[0]).map(|(count, _)| count), Some(0));
        assert_eq!(
            rows(&ran.outcomes[1]).map(|(count, _)| count),
            Some(1),
            "{:?}",
            error(&ran.outcomes[1])
        );
        let x = &ran.probes[1];
        assert_eq!(x.instantiated, Instantiated::Never, "X is never visited");
        let y = &ran.probes[0];
        assert_eq!(
            y.instantiated == Instantiated::Slot,
            shared,
            "Y runs only when F reads it"
        );
        assert_eq!(ran.dormant, 0);
        assert_eq!(ran.cleanup, Ok(()));
        assert_eq!(ran.finished, Ok(()));
    }
}

/// The recogniser's self-join anti-join shape over `source_location`: its select runs on the ordered-existence route.
fn routed() -> Plan {
    let schema = kinds(Relation::SourceLocation);
    let number = ColumnIndex(
        u32::try_from(
            schema
                .iter()
                .position(|kind| *kind == CellKind::Number)
                .expect("a numeric column"),
        )
        .unwrap_or(0),
    );
    plan(vec![
        select(Input::Relation(Relation::SourceLocation), vec![]),
        step(Operation::Join {
            left: at(0),
            right: at(0),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: number,
                },
                JoinColumn {
                    side: Side::Right,
                    column: number,
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
        step(Operation::Select {
            from: at(1),
            comparisons: vec![PlanComparison {
                test: Test::LessThan,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(1)),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: vec![],
        }),
        step(Operation::Project {
            from: at(2),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::Antijoin {
            left: at(0),
            right: at(3),
            left_on: vec![number],
            right_on: vec![ColumnIndex(0)],
        }),
    ])
}

/// R1.7, a never-polled absent root: rule A's absent root is read by an interior join of admitted rule F that is itself
/// absent on the file, so the join publishes without polling A and its release reaches A untouched. The cascade stops at
/// A: none of A's uncounted inputs is released, and nothing is refused.
#[test]
fn a_cascade_stops_at_an_absent_root_that_never_ran() {
    let a = || {
        vec![
            select(Input::Relation(Relation::Comment), vec![]),
            join_on_file(0, 0, 5),
            select(Input::Relation(Relation::Function), vec![equal(TEXT, "y")]),
            join_on_file(1, 2, 5),
        ]
    };
    let mut f = a();
    f.push(select(
        Input::Relation(Relation::Function),
        vec![equal(TEXT, "g")],
    ));
    f.push(join_on_file(3, 4, 5));
    f.push(select(
        Input::Relation(Relation::Comment),
        vec![equal(TEXT, "x")],
    ));
    f.push(step(Operation::Union {
        left: at(5),
        right: at(6),
    }));
    let prepared = prepare(vec![plan(f), plan(a())]);
    let regions = regions_of(&prepared, 1);
    let ran = run(
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent(), run_intent()],
        &[],
        &regions,
    );
    assert_eq!(rows(&ran.outcomes[0]).map(|(count, _)| count), Some(1));
    assert_eq!(rows(&ran.outcomes[1]).map(|(count, _)| count), Some(0));
    assert_eq!(
        ran.probes[1].instantiated,
        Instantiated::Never,
        "A's input was never released or run"
    );
    assert_eq!(ran.cleanup, Ok(()));
    assert_eq!(ran.finished, Ok(()));
}

/// R1.8: a program with an ordered-existence route publishes its absent root without entering the route or demanding any
/// of its inputs.
#[test]
fn an_absent_root_of_a_routed_program_never_enters_the_route() {
    let prepared = prepare(vec![routed()]);
    let regions = regions_of(&prepared, 0);
    assert!(
        regions.iter().any(|region| matches!(
            prepared.program().regions()[region.0].route(),
            ryo_execution_domain::domain::executable_program::types::Route::OrderedExistence(_)
        )),
        "the fixture compiles a routed region"
    );
    let ran = run(
        &prepared,
        &mut facts(&[], &[]),
        &[run_intent()],
        &[],
        &regions,
    );
    assert!(
        matches!(ran.outcomes[0], Ok(RootOutcome::Complete(_))),
        "{:?}",
        error(&ran.outcomes[0])
    );
    let root = *regions.last().expect("the routed rule has steps");
    for (region, probe) in regions.iter().zip(&ran.probes) {
        if *region != root {
            assert_eq!(
                probe.instantiated,
                Instantiated::Never,
                "{region:?} was never demanded"
            );
        }
    }
    assert_eq!(ran.presence_absent, 1);
    assert_eq!(ran.finished, Ok(()));
}

/// R1.9: counts report what ran: an absent rule's steps report zero where their region did not run on the file, and the
/// count of a region it shares with a rule that ran; the other rule's counts are unchanged.
#[test]
fn counts_report_only_what_ran() {
    let shared = || select(Input::Relation(Relation::Comment), vec![]);
    let absent = plan(vec![
        shared(),
        select(Input::Relation(Relation::Function), vec![equal(TEXT, "y")]),
        join_on_file(0, 1, 5),
    ]);
    let ran_rule = plan(vec![shared(), select(at(0), vec![equal(TEXT, "x")])]);
    let prepared = prepare(vec![absent, ran_rule]);
    let intents = [run_intent(), run_intent()];
    let ran = run(&prepared, &mut facts(&["x"], &["f"]), &intents, &[], &[]);
    let alone = run(
        &prepare(vec![plan(vec![
            shared(),
            select(at(0), vec![equal(TEXT, "x")]),
        ])]),
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        &[],
        &[],
    );
    assert_eq!(
        ran.fed[0][1..],
        [0, 0],
        "the absent rule's own regions did not run"
    );
    assert_eq!(
        ran.fed[1], alone.fed[0],
        "the other rule's counts are unchanged"
    );
}

/// R1.10: an inapplicable rule keeps its skipped root and is never consulted; a file with a partial read runs with the gate
/// off, every root in full.
#[test]
fn inapplicable_rules_and_partial_files_keep_their_full_routes() {
    let prepared = prepare(vec![rule_b()]);
    let b = regions_of(&prepared, 0);
    let skipped = run(
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)],
        &[],
        &b,
    );
    assert_eq!(skipped.presence_absent, 0);
    assert!(
        skipped
            .probes
            .iter()
            .all(|probe| probe.instantiated == Instantiated::Never)
    );
    let partial = Partial {
        relation: Relation::Function,
        cause: ryo_values::domain::stored_facts::types::PartialCause("unparsed".into()),
        lacking: ryo_execution_domain::domain::coverage::types::Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(0)),
    };
    let gated_off = run(
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        std::slice::from_ref(&partial),
        &b,
    );
    assert_eq!(gated_off.presence_absent, 0);
    assert!(
        gated_off
            .probes
            .iter()
            .take(b.len() - 1)
            .any(|probe| probe.instantiated == Instantiated::Slot),
        "with the gate off, B's inputs run"
    );
}

/// R1.11: a test whose comparison errs on a row the pipeline drops before it holds; the rule's answer equals its full
/// run's.
#[test]
fn a_test_error_holds_and_the_answer_is_unchanged() {
    let erring = plan(vec![
        select(Input::Relation(Relation::Comment), vec![equal(TEXT, "x")]),
        select(at(0), vec![compare(TEXT, Test::Matches, "(")]),
    ]);
    let prepared = prepare(vec![erring]);
    let ran = run(
        &prepared,
        &mut facts(&["y"], &[]),
        &[run_intent()],
        &[],
        &[],
    );
    assert_eq!(rows(&ran.outcomes[0]).map(|(count, _)| count), Some(0));
    assert_eq!(ran.finished, Ok(()));
}

/// R1.12: a step over an invalid schema reports its error whatever its formula says.
#[test]
fn a_schema_error_under_a_false_formula_is_reported() {
    let invalid = plan(vec![
        select(Input::Relation(Relation::Comment), vec![equal(TEXT, "y")]),
        select(at(0), vec![equal(99, "z")]),
        step(Operation::Project {
            from: at(1),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }),
    ]);
    let prepared = prepare(vec![invalid]);
    let ran = run(
        &prepared,
        &mut facts(&["x"], &[]),
        &[run_intent()],
        &[],
        &[],
    );
    assert!(
        error(&ran.outcomes[0]).is_some(),
        "the schema error is reported"
    );
}

/// R1.3 and R1.13: everything downstream of a bound observation may have rows in any file. A consumer of a producer's
/// named output is never absent, and a Replace over a Symbol column fed by it, where a name that left its file can
/// arrive, is unconditional even when its other conjunct cannot match.
#[test]
fn consumers_of_a_bound_observation_are_never_absent() {
    use ryo_execution_domain::domain::observation_schedule::types::{ObservationRequest, ResolvedObservationBinding, ResolvedObservationProducer};
    use ryo_values::domain::observations::types::{
        BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationName, PlanObservation,
    };
    use ryo_values::domain::plan_parts::types::ObservationBindingId;
    use ryo_values::domain::records::types::{ObservationIndex, ObservationReadinessLayout};
    let columns = vec![
        ObservationColumn {
            name: ObservationColumnName("text".into()),
            kind: CellKind::Symbol,
        },
        ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        },
    ];
    let schema = BoundObservationInput {
        name: ObservationBindingName("named".into()),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    };
    let mut producer = plan(vec![step(Operation::Select {
        from: Input::Relation(Relation::Comment),
        comparisons: vec![equal(TEXT, "y")],
        kept: vec![ColumnIndex(TEXT), ColumnIndex(2)],
    })]);
    producer.observations.push(PlanObservation {
        name: ObservationName("out".into()),
        source: at(0),
        columns,
        scope: StepScope::PerFile,
    });
    let mut consumer = plan(vec![
        select(Input::BoundObservation(ObservationBindingId(0)), vec![]),
        step(Operation::Replace {
            from: at(0),
            text: ColumnIndex(0),
            pattern: LiteralText("a".into()),
            replacement: LiteralText("b".into()),
        }),
        select(Input::Relation(Relation::Function), vec![equal(TEXT, "y")]),
        step(Operation::Join {
            left: at(1),
            right: at(2),
            left_on: vec![ColumnIndex(1)],
            right_on: vec![ColumnIndex(2)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(2),
            }],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
    ]);
    consumer.observation_inputs.push(schema.clone());
    let plans = vec![producer, consumer];
    let bindings = vec![
        vec![],
        vec![ResolvedObservationBinding {
            input: ObservationBindingId(0),
            schema,
            producers: vec![ResolvedObservationProducer::Output {
                rule: RuleIndex(0),
                output: ObservationIndex(0),
            }],
        }],
    ];
    let operators = crate::application::file_dag::service::prepare(
        &plans,
        &[vec![], vec![]],
        &[],
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
    )
    .expect("the observation plans prepare");
    let graph = ryo_execution_domain::domain::file_program::service::compile_prepared_observations(
        &plans,
        &operators,
        ObservationRequest::Bound {
            layout: ObservationReadinessLayout(Digest([7; 16])),
            bindings: &bindings,
        },
    );
    let presence = ryo_execution_domain::domain::executable_program::presence::service::of_graph(
        &graph,
        &|_| {
            ryo_execution_domain::domain::executable_program::presence::types::RelationScope::CurrentFile
        },
        ryo_execution_domain::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    );
    let consumer = graph.program_nodes[1]
        .iter()
        .map(|node| node.as_ref().expect("every consumer step has a node"))
        .collect::<Vec<_>>();
    for node in [consumer[0], consumer[1], consumer[3]] {
        assert_eq!(
            presence.nodes[node.0],
            ryo_execution_domain::domain::executable_program::presence::service::ALWAYS,
            "{node:?} reads the observation"
        );
    }
    assert_ne!(
        presence.nodes[consumer[2].0],
        ryo_execution_domain::domain::executable_program::presence::service::ALWAYS,
        "the unrelated select keeps its test"
    );
}
