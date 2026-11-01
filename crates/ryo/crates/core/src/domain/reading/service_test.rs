//! The `reading` service's cases, on values alone.

use crate::domain::table_words::service::pack;
use crate::domain::table_words::types::WordSlice;
use crate::outbound::matching::interface::MatchingPort as _;
use ryo_language::domain::relations::service::to_bits;
use ryo_language::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, FileId, Integer, Offset, Relation, Value as Cell,
    WorkerNumber,
};
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as PlanInput, InputNumber, LiteralText, Operand, QueryLiteral,
    RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use ryo_values::domain::records::types::Kept;
use ryo_values::domain::runs::types::MemoryBytes;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket, KeptWords};

use super::service::{
    begin, capture_frontier, engine_asked, evaluate_rule, first_buckets, frontier_bucket,
    prepare_frontier, settle, start_partials,
};
use super::types::EngineRows;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::coverage::types::{Recovery, RuleIndex};
use crate::domain::finish::types::{ColumnNumber, FileLine, RuleNumber};
use crate::domain::lowering::types::Counting;
use crate::domain::operator::types::TreeSteps;
use crate::domain::placement::types::{PlacedScopes, ScopePlacement};
use crate::domain::render::types::{RowNumber, RowOrder};
use crate::domain::rules::errors::RulesError;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{ParameterBinding, Table};
use crate::domain::visit::types::{Noting, RoleRead};
use crate::domain::workers::service::tables_for;
use crate::domain::workers::types::{Tables, VisitSetting};
use crate::outbound::matching::fixtures::fake::MatchingFake;
use ryo_values::domain::records::service::keep;
use ryo_values::domain::records::types::Cell as Typed;
use ryo_values::domain::records::types::CellText;
use ryo_values::domain::runs::types::RuleId;

/// What the last run kept, for a run that uses nothing of it.
static NOTHING_STORED: std::sync::LazyLock<ryo_values::domain::stores::types::Stored> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::Stored::default);

/// What the files of the last run handed, for a run that uses nothing of it.
static NOTHING_HANDED: std::sync::LazyLock<ryo_values::domain::stores::types::StoredHanded> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::StoredHanded::default);

/// No bucket of what the files handed, for a run that reads none.
static NOTHING_WANTED: ryo_values::domain::stores::types::BucketsWanted =
    ryo_values::domain::stores::types::BucketsWanted::These(Vec::new());

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Names the bucket of the third input of a rule whose frontier starts in bucket 3.
    FrontierBucket,
    /// Readies a worker that holds six bytes of text for its first file.
    Begin,
    /// Says which engine rows the rules ask for when only the line count is wanted.
    EngineAsked,
    /// Notes the relations read in part when the tree is a recovery from syntax errors.
    StartPartials,
    /// Runs a rule that keeps two functions, and renders them.
    EvaluateRule,
    /// Runs a rule that compares a column the rows do not have.
    EvaluateRefused,
    /// Hands over the two rows a rule's frontier holds, and keeps a copy.
    LeaveFrontier,
    /// Puts the findings of a file in their order.
    Settle,
    /// Numbers the first bucket of the frontier of two rules.
    FirstBuckets,
}

/// What a row of the cases does.
type Input = Scenario;
/// A count, or several written one after the other as digits.
type Value = u64;
type Failure = RulesError;

const SOURCE: &[u8] = b"mainother";

const KINDS: &[CellKind] = &[CellKind::Symbol, CellKind::File];

fn count(number: usize) -> u64 {
    u64::try_from(number).unwrap_or(u64::MAX)
}

fn asked(rows: EngineRows) -> u64 {
    u64::from(rows == EngineRows::Asked)
}

/// A plan that keeps the functions named `main` (or compares a column they lack) and projects their file.
fn plan(comparing: u32, frontier: usize) -> Plan {
    let select = PlanStep {
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::Function),
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(comparing)),
                right: if comparing == 0 {
                    Operand::Literal(QueryLiteral::Text(LiteralText("main".into())))
                } else {
                    Operand::Literal(QueryLiteral::Number(Integer(1)))
                },
                origin: None,
            }],
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    };
    let project = PlanStep {
        operation: Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![ColumnIndex(1)],
            distinct: RepeatedRows::Dropped,
        },
        scope: StepScope::PerFile,
    };
    Plan {
        parameters: Vec::new(),
        steps: vec![select, project],
        emit: Emit {
            step: StepId(1),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: vec![PlanInput::Step(StepId(0)); frontier],
        comparison_origins: Vec::new(),
    }
}

fn worker(plans: &[Plan]) -> Tables {
    tables_for(
        WorkerNumber(0),
        plans,
        TreeSteps::Lowered,
        Counting::DoesNotCount,
    )
}

fn placed() -> PlacedScopes {
    PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: MatchingFake.compile(&[]).expect("no glob is a bad one"),
    }
}

fn setting<'a>(
    placed: &'a PlacedScopes,
    plans: &'a [Plan],
    first: &'a [Bucket],
    ids: &'a [RuleId],
    bindings: &'a [Vec<ParameterBinding>],
) -> VisitSetting<'a> {
    VisitSetting {
        unknown: &[],
        plans,
        bindings,
        wanted: &[],
        paths: &[],
        first_bucket: first,
        ids,
        applicability: &[],
        applicability_can_gate: true,
        placed,
        records: &[],
        replaying: crate::domain::workers::types::Replaying::Findings,
        stored: &NOTHING_STORED,
        handed: &NOTHING_HANDED,
        wanted_buckets: &NOTHING_WANTED,
        budget: MemoryBytes(0),
        noted: Noting::Unwanted,
        roled: RoleRead::Skips,
        needs: &[],
        answering: &crate::domain::render::types::Answering::Findings,
        keeping: crate::domain::visit::types::Keeping::Keeps,
    }
}

/// Runs the one rule of `plan` on a file whose functions are two named `main`, in files 10 and 11, and one named `other`.
fn evaluate(plan: Plan) -> Result<Value, Failure> {
    let plans = [plan];
    let placed = placed();
    let mut tables = worker(&plans);
    let names = [(0, 4), (0, 4), (4, 9)].map(|(start, end)| {
        to_bits(Cell::Symbol(intern(
            &mut tables.symbols,
            ByteSlice(SOURCE),
            Offset(start),
            Offset(end),
        )))
        .0
    });
    let files = [10, 11, 11].map(|file| to_bits(Cell::File(FileId(file))).0);
    let columns = [FactColumn(names.to_vec()), FactColumn(files.to_vec())];
    let relations = |relation: Relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    tables.bytes = FileBytes(SOURCE.to_vec());
    let (ids, bindings) = ([RuleId("rule".into())], [Vec::new()]);
    let setting = setting(&placed, &plans, &[], &ids, &bindings);
    evaluate_rule(
        &mut tables,
        &setting,
        RuleIndex(0),
        FileId(10),
        None,
        &relations,
    )?;
    Ok(count(tables.rendered.order.len()) * 10
        + u64::from(tables.rendered.text.starts_with("rule")))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::FrontierBucket => Ok(u64::from(frontier_bucket(Bucket(3), InputNumber(2)).0)),
        Scenario::Begin => {
            let mut tables = worker(&[]);
            tables.arena = ArenaBytes(vec![0; 6]);
            tables.captured = vec![Kept {
                bucket: Bucket(1),
                words: KeptWords(PackedWords(vec![1])),
            }];
            let base = begin(&mut tables);
            Ok(u64::from(base.0) * 10
                + u64::from(tables.reads.0)
                + count(tables.captured.len()) * 100)
        }
        Scenario::EngineAsked => {
            let engine = engine_asked(&[Relation::FilesystemLineCount]);
            Ok(asked(engine.file) * 100 + asked(engine.lines) * 10 + asked(engine.any))
        }
        Scenario::StartPartials => {
            let mut tables = worker(&[]);
            let wanted = [Relation::Defines, Relation::FilesystemFile];
            start_partials(&mut tables, &[], Recovery::SyntaxErrors, &wanted);
            Ok(count(tables.partials.len()) * 10 + count(tables.pool.partial.len()))
        }
        Scenario::EvaluateRule => evaluate(plan(0, 0)),
        Scenario::EvaluateRefused => evaluate(plan(9, 0)),
        Scenario::LeaveFrontier => {
            let plans = [plan(0, 1)];
            let placed = placed();
            let first = [Bucket(4)];
            let setting = setting(&placed, &plans, &first, &[], &[]);
            let mut tables = worker(&plans);
            let mut rows = Table {
                kinds: vec![CellKind::Number],
                ..Table::default()
            };
            for number in [1, 2] {
                push_row(&mut rows, &[Cell::Number(Integer(number))]);
            }
            tables.pool.frontier[0] = rows;
            let mut seen = Vec::new();
            if let Some(bucket) = prepare_frontier(
                &mut tables,
                &setting,
                RuleIndex(0),
                FileId(0),
                InputNumber(0),
            ) {
                seen.push((bucket, tables.packed.clone()));
                capture_frontier(&mut tables, setting.answering, bucket);
            }
            let kept = tables.captured.first();
            // What is kept is the words of the table, packed.
            let same = kept.is_some_and(|kept| {
                seen.first().is_some_and(|each| each.1 == kept.words.0)
                    && pack(WordSlice(&tables.words.0)) == kept.words.0
            });
            Ok(u64::from(seen[0].0.0) * 100 + count(tables.captured.len()) * 10 + u64::from(same))
        }
        Scenario::Settle => {
            let mut tables = worker(&[]);
            for path in ["z", "a"] {
                let rendered = &mut tables.rendered;
                let id = keep(&mut rendered.text, Typed::Text(CellText("r")));
                rendered.cells.push(Some(id));
                let file = keep(&mut rendered.text, Typed::File(CellText(path)));
                rendered.cells.push(Some(file));
                rendered
                    .cells
                    .push(Some(crate::domain::render::service::key_count(
                        crate::domain::render::types::KeyCount(1),
                    )));
                let key = keep(&mut rendered.text, Typed::Text(CellText("k")));
                rendered.cells.extend([Some(key), None, None]);
                crate::domain::render::service::end_row(rendered);
            }
            tables.rendered.order = [0, 1]
                .map(|row| RowOrder {
                    rule: RuleNumber(0),
                    line: FileLine(0),
                    column: ColumnNumber(0),
                    row: RowNumber(row),
                })
                .to_vec();
            settle(
                &mut tables,
                &crate::domain::render::types::Answering::Findings,
            );
            Ok(u64::from(tables.rendered.order[0].row.0))
        }
        Scenario::FirstBuckets => {
            let first = first_buckets(&[plan(0, 2), plan(0, 1)]);
            Ok(u64::from(first[1].0) * 10 + u64::from(first[0].0))
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_input_of_the_frontier_waits_in_the_bucket_after_the_ones_before_it"),
        word: Word::Succeeds,
        input: || Scenario::FrontierBucket,
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("a_file_is_counted_and_its_text_begins_where_the_worker_left_off"),
        word: Word::Succeeds,
        input: || Scenario::Begin,
        expect: Expect::Value(61),
    },
    Case {
        name: CaseName("a_rule_that_asks_for_the_line_count_asks_for_the_engine_rows"),
        word: Word::Succeeds,
        input: || Scenario::EngineAsked,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_recovery_from_syntax_errors_is_partial_in_the_relations_of_code"),
        word: Word::Succeeds,
        input: || Scenario::StartPartials,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_rule_is_run_on_the_rows_of_the_file_and_its_findings_rendered"),
        word: Word::Succeeds,
        input: || Scenario::EvaluateRule,
        expect: Expect::Value(21),
    },
    Case {
        name: CaseName("a_rule_an_operator_refuses_fails"),
        word: Word::Refuses,
        input: || Scenario::EvaluateRefused,
        expect: Expect::Refusal(|error| {
            if matches!(error, RulesError::Failed { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName(
            "the_rows_of_the_frontier_are_handed_over_and_a_copy_kept_when_the_run_keeps_its_findings",
        ),
        word: Word::Succeeds,
        input: || Scenario::LeaveFrontier,
        expect: Expect::Value(411),
    },
    Case {
        name: CaseName("findings_are_put_in_the_order_of_their_keys"),
        word: Word::Succeeds,
        input: || Scenario::Settle,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_findings_have_bucket_zero_and_each_frontier_the_buckets_after"),
        word: Word::Succeeds,
        input: || Scenario::FirstBuckets,
        expect: Expect::Value(31),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// A direct resident self-join used by both real rule evaluations in the cache proof.
fn duplicate_resident_join_plan() -> Plan {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    Plan {
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Join {
                left: PlanInput::Relation(Relation::Function),
                right: PlanInput::Relation(Relation::Function),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
                filter: Vec::new(),
                view: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(1),
            key: vec![ColumnIndex(0)],
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

#[test]
fn two_real_rule_evaluations_share_only_the_index_and_keep_ordered_duplicate_rows() {
    use crate::domain::stored_indexes::service::{LIMIT, new};
    let join = duplicate_resident_join_plan();
    let plans = [join.clone(), join];
    let mut tables = worker(&plans);
    tables.bytes = FileBytes(SOURCE.to_vec());
    let main = intern(&mut tables.symbols, ByteSlice(SOURCE), Offset(0), Offset(4));
    let other = intern(&mut tables.symbols, ByteSlice(SOURCE), Offset(4), Offset(9));
    let columns = [
        FactColumn(vec![
            u64::from(main.0),
            u64::from(other.0),
            u64::from(main.0),
        ]),
        FactColumn(vec![10, 11, 12]),
    ];
    let reads = std::cell::Cell::new(0_u32);
    let placed = placed();
    let ids = [RuleId("first".into()), RuleId("second".into())];
    let bindings = [Vec::new(), Vec::new()];
    let setting = setting(&placed, &plans, &[], &ids, &bindings);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("audited physical generation");
    evaluate_rule(
        &mut tables,
        &setting,
        RuleIndex(0),
        FileId(10),
        Some(&mut cache),
        &|relation| {
            reads.set(reads.get() + 1);
            (relation == Relation::Function).then_some(ColumnsView {
                kinds: KINDS,
                columns: &columns,
            })
        },
    )
    .expect("first rule");
    // Shape inference currently computes direct relation inputs with real stored rows.
    // One acquisition belongs to layout and one to execution, not to each of the five hits.
    assert_eq!(
        (cache.builds, cache.reuses),
        (1, 1),
        "first-rule layout plus execution"
    );
    let first = tables
        .pool
        .steps
        .first()
        .expect("kept emitted join")
        .clone();
    let reads_after_first = reads.get();
    evaluate_rule(
        &mut tables,
        &setting,
        RuleIndex(1),
        FileId(10),
        Some(&mut cache),
        &|relation| {
            reads.set(reads.get() + 1);
            (relation == Relation::Function).then_some(ColumnsView {
                kinds: KINDS,
                columns: &columns,
            })
        },
    )
    .expect("second rule");
    assert_eq!(tables.pool.steps.first(), Some(&first));
    assert_eq!(
        first.count.0, 5,
        "duplicate keys preserve the ordered join multiset"
    );
    assert_eq!(
        (cache.builds, cache.reuses),
        (1, 3),
        "two rule layouts and two executions still build one physical index"
    );
    assert!(
        reads.get() > reads_after_first,
        "normal provider reads remain observable on a hit"
    );
}
