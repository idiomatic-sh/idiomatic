//! The `assembling` service's cases, on query text read by the real syntax adapter.

use ryo_relations::domain::relations::types::{CellKind, Relation};

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::assembling::service::lower;
use crate::domain::lowering::errors::LoweringError;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::selects;
use ryo_values::domain::hosted::types::ColumnLabel;
use ryo_values::domain::hosted::types::HostedColumn;
use ryo_values::domain::hosted::types::HostedName;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::program::types::QueryView;

/// The query text a row gives.
type Text = &'static str;
/// The operations of the plan's steps, in order, named.
type Operations = &'static str;
/// What the lowering refuses a row with.
type Failure = LoweringError;

/// What a row asks of the part.
#[derive(Clone, Copy)]
enum Scenario {
    /// Lower the text the row gives.
    Text(Text),
    /// A check that drives the part itself and asserts what it answers.
    Check(fn()),
}

/// What a row answers.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// The lowering answered this.
    Lowered(Operations),
    /// The check ran and held every assertion it makes.
    Checked,
}

type Input = Scenario;
type Value = Answer;

const PROJECTED: &str = "let a = select name, file as \"f\" from \"function\" where name = \"main\"; emit a subject f key(f);";
const UNKNOWN_PARAMETER_TYPE: &str =
    "param limit: nothing; let a = select * from \"function\"; emit a subject file key(span);";
/// The rows the host declared, named as ryo's own are: they are the same for every file, so the step that reads them runs after the files.
const HOSTED: &str = "let p = select * from \"host.placed\"; emit p subject path key(spec);";

const CAPTURED_THEN_FILTERED: &str = "let c = capture(from=\"function\", path=file, pattern=\"{*d}/x.rs\", captures=[d]); let f = select * from c where name = \"main\"; emit f subject file key(span);";
const CAPTURED_THEN_FILTERED_ON_THE_CAPTURE: &str = "let c = capture(from=\"function\", path=file, pattern=\"{*d}/x.rs\", captures=[d]); let f = select * from c where d = \"src\"; emit f subject file key(span);";
const COLUMNS_ASKED_ACROSS_FILES: &str = "let a = select name, file from \"function\"; let b = select name, file from \"function\"; let j = select l.\"file\" as \"file\" from a as l join b as r on l.\"name\" = r.\"name\"; emit j subject file key(file);";
const COLUMNS_OF_A_TREE_ASKED_ACROSS_FILES: &str = "let a = select name, id from \"function\"; let m = select l.\"id\" as \"id\", r.\"child\" as \"child\" from a as l join \"syntax-child\" as r on l.\"name\" = r.\"parent\"; emit m subject id key(child);";

/// The functions of each name counted by the files they are in: only the name and the file are asked of the rows.
const DISTINCT_COUNT: &str = "let f = select * from \"function\"; let c = group-count(from=f, keys=[\"name\"], count=\"n\", distinct=\"file\"); emit c subject name key(n);";

/// The rows of this part's cases.
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Text(text) => lower(text).map(Answer::Lowered),
        Scenario::Check(check) => {
            check();
            Ok(Answer::Checked)
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_and_a_projection_are_two_steps"),
        word: Word::Succeeds,
        input: || Scenario::Text(PROJECTED),
        expect: Expect::Value(Answer::Lowered("select project")),
    },
    Case {
        name: CaseName("a_relation_the_host_declared_is_a_step_that_runs_once_after_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Text(HOSTED),
        expect: Expect::Value(Answer::Lowered("hosted* select*")),
    },
    Case {
        name: CaseName("a_filter_on_the_rows_a_capture_reads_is_made_before_the_capture"),
        word: Word::Succeeds,
        input: || Scenario::Text(CAPTURED_THEN_FILTERED),
        expect: Expect::Value(Answer::Lowered("select capture select")),
    },
    Case {
        name: CaseName("a_filter_on_the_columns_a_capture_made_stays_after_it"),
        word: Word::Succeeds,
        input: || Scenario::Text(CAPTURED_THEN_FILTERED_ON_THE_CAPTURE),
        expect: Expect::Value(Answer::Lowered("capture select")),
    },
    Case {
        name: CaseName(
            "the_rows_a_file_hands_over_are_cut_to_the_columns_the_steps_after_the_files_read",
        ),
        word: Word::Succeeds,
        input: || Scenario::Text(COLUMNS_ASKED_ACROSS_FILES),
        expect: Expect::Value(Answer::Lowered("project project join*")),
    },
    Case {
        name: CaseName(
            "a_tree_relation_read_across_files_is_cut_to_the_columns_asked_by_a_step_before_the_join",
        ),
        word: Word::Succeeds,
        input: || Scenario::Text(COLUMNS_OF_A_TREE_ASKED_ACROSS_FILES),
        expect: Expect::Value(Answer::Lowered("select select join*")),
    },
    Case {
        name: CaseName(
            "rows_handed_over_for_the_columns_the_finish_asks_are_cut_to_those_without_repeats",
        ),
        word: Word::Succeeds,
        input: || Scenario::Text(DISTINCT_COUNT),
        expect: Expect::Value(Answer::Lowered("select project group-count*")),
    },
    Case {
        name: CaseName("a_parameter_of_a_type_the_language_lacks_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Text(UNKNOWN_PARAMETER_TYPE),
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownParameterType { .. }))
        }),
    },
    Case {
        name: CaseName(
            "actual_compilation_removes_global_closure_for_partition_and_source_keyed_lookups",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::Check(
                actual_compilation_removes_global_closure_for_partition_and_source_keyed_lookups,
            )
        },
        expect: Expect::Value(Answer::Checked),
    },
    Case {
        name: CaseName(
            "authored_projected_edge_and_shared_consumers_lower_to_demand_in_the_real_pipeline",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::Check(
                authored_projected_edge_and_shared_consumers_lower_to_demand_in_the_real_pipeline,
            )
        },
        expect: Expect::Value(Answer::Checked),
    },
];

/// What an operation is, for the answer.
fn operation_name(operation: &Operation) -> &'static str {
    match operation {
        Operation::Select { .. } => "select",
        Operation::Project {
            distinct: RepeatedRows::Kept,
            ..
        } => "project-keyed",
        Operation::Project { .. } => "project",
        Operation::Join { .. } => "join",
        Operation::Replace { .. } => "replace",
        Operation::SpanBetween { .. } => "span-between",
        Operation::SpanBounds { .. } => "span-bounds",
        Operation::SourceMatches { .. } => "source-matches",
        Operation::OrderedMatch { .. } => "ordered-match",
        Operation::OrderedDfs { .. } => "ordered-dfs",
        Operation::GroupCount { .. } => "group-count",
        Operation::GroupDigest { .. } => "group-digest",
        Operation::Antijoin { .. } => "antijoin",
        Operation::Union { .. } => "union",
        Operation::Capture { .. } => "capture",
        Operation::Extend { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. } => "other",
        Operation::Rows { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::Hosted { .. } => "hosted",
        Operation::Axis { .. } => "axis",
    }
}

/// Lowers the text and names the operations of the plan, but the answer must be one of the known lists.
fn lower(text: Text) -> Result<Operations, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("the row's text is readable");
    let plan = lower(
        &program,
        &[
            Relation::Function,
            Relation::SyntaxChild,
            Relation::SyntaxTree,
            Relation::WorkspaceMember,
        ],
        &[HostedRelation {
            name: HostedName("host.placed".into()),
            columns: vec![
                HostedColumn {
                    name: ColumnLabel("path".into()),
                    kind: CellKind::File,
                },
                HostedColumn {
                    name: ColumnLabel("spec".into()),
                    kind: CellKind::Symbol,
                },
            ],
        }],
    )?;
    let names: Vec<String> = plan
        .steps
        .iter()
        .map(|step| {
            format!(
                "{}{}",
                operation_name(&step.operation),
                if step.scope == StepScope::CrossFile {
                    "*"
                } else {
                    ""
                }
            )
        })
        .collect();
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Ok(Box::leak(names.join(" ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn actual_compilation_removes_global_closure_for_partition_and_source_keyed_lookups() {
    let text = r#"
        let roots = select id, file from "function";
        let reach = closure(edges="semantic-child", source="parent", destination="child", keys=["file"]);
        let found = select l.id as "id", l.file as "file", r.child as "child"
            from roots as l join reach as r on l.id = r.parent and l.file = r.file;
        emit found subject file key(id, child);
    "#;
    let program = winnow().read(QueryView(text)).expect("query syntax");
    let plan = lower(
        &program,
        &[Relation::Function, Relation::SemanticChild],
        &[],
    )
    .expect("typed plan");
    assert!(
        plan.steps.iter().any(|step| matches!(
            &step.operation,
            Operation::Join {
                right: ryo_values::domain::plan_parts::types::Input::Relation(
                    Relation::SemanticChild
                ),
                reach: Some(_),
                ..
            }
        )),
        "demand must survive the real optimization pipeline: {:?}",
        plan.steps
    );
    assert!(
        !plan
            .steps
            .iter()
            .any(|step| matches!(step.operation, Operation::Closure { .. })),
        "unused all-source closure must not remain kept"
    );
}

fn authored_projected_edge_and_shared_consumers_lower_to_demand_in_the_real_pipeline() {
    // The hot-rule shape projects semantic-child before closure. Its columns are Shared, so the
    // actual compiler inserts a distinct Project; a raw-relation-only test does not exercise this.
    let text = r#"
        let roots = select id, file from "function";
        let edges = select parent as origin, child as destination from "semantic-child";
        let descendants = closure(edges=edges, source="origin", destination="destination");
        let first = select l.id as id, l.file as file, r.destination as child from roots as l join descendants as r on l.id = r.origin;
        let second = select l.id as id, l.file as file, r.destination as child from roots as l join descendants as r on l.id = r.origin;
        let all = union(left=first, right=second);
        emit all subject file key(id,child);
    "#;
    let program = winnow()
        .read(QueryView(text))
        .expect("authored edge syntax");
    let plan = lower(
        &program,
        &[Relation::Function, Relation::SemanticChild],
        &[],
    )
    .expect("typed plan");
    assert!(
        !plan
            .steps
            .iter()
            .any(|step| matches!(step.operation, Operation::Closure { .. })),
        "projected hot-rule shape must not retain global closure: {:?}",
        plan.steps
    );
    assert!(
        plan.steps
            .iter()
            .filter(|step| matches!(step.operation, Operation::Join { reach: Some(_), .. }))
            .count()
            >= 2,
        "all shared keyed consumers must be rewritten together"
    );
}
