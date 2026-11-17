//! The rows each node's kernels read from each of its inputs when it was made, through `plan` and `run` only: every kind of
//! node, in its inputs' order, a node read by two inputs in both, and none once released.
use super::super::super::fixtures::cases::{DEFINES, input_of};
use super::super::super::fixtures::relational::node;
use super::super::super::fixtures::{TableInputs, TextResources, planned, ran};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::text::text_program;
use crate::domain::physical_plan::fixtures::{Step, defines_rows, over_nodes};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId, Resources as _,
};
use ryo_language::domain::semantics::types::RenderOutcome;
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{FileId, Offset, Position, Relation, Span};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Input as StepInput, StepScope,
};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::CellText;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    EveryKind,
}
type Input = Scenario;
/// One line per node, its kind, its rows fed from each input and the fault it answered, then what the last reports once
/// released.
type Value = Cow<'static, str>;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("fed_follows_the_logical_inputs"),
    word: Word::Succeeds,
    input: || Scenario::EveryKind,
    // Each node of the text program and of the kinds it lacks, over three `DEFINES` rows and the four-node small tree:
    // the rows its kernels read from each input, in its inputs' order. The tree's edges are three rows (the root has no
    // edge); a node read by two inputs counts in both; a node released reports none.
    expect: Expect::Value(Cow::Borrowed(FED)),
}];

/// What `fed_follows_the_logical_inputs` reads off a run.
const FED: &str = "0: Select [3, 0, 0]\n\
                   1: Select [3, 0, 0]\n\
                   2: Project [2, 0, 0]\n\
                   3: Project [1, 0, 0]\n\
                   4: Union [2, 1, 0]\n\
                   5: Intersection [2, 2, 0]\n\
                   6: Difference [2, 2, 0]\n\
                   7: Antijoin [3, 1, 0]\n\
                   8: Extend [2, 0, 0]\n\
                   9: Join [2, 3, 0]\n\
                   10: Join [2, 3, 0]\n\
                   11: GroupCount [3, 0, 0]\n\
                   12: GroupCount [3, 0, 0]\n\
                   13: GroupDigest [3, 0, 0]\n\
                   14: Closure [3, 0, 0]\n\
                   15: Join [2, 3, 0]\n\
                   16: Axis [2, 3, 3]\n\
                   17: OrderedDfs [2, 2, 0]\n\
                   18: Cycles [3, 0, 0]\n\
                   19: Select [4, 0, 0]\n\
                   20: Capture [3, 0, 0]\n\
                   21: Rank [3, 0, 0] Unbound\n\
                   22: Replace [3, 0, 0]\n\
                   23: OrderedMatch [3, 3, 0] Derived(ShapeMismatch)\n\
                   24: Words [3, 0, 0]\n\
                   25: SpanBetween [3, 0, 0]\n\
                   26: SourceMatches [3, 0, 0]\n\
                   27: RenderSyntax [3, 0, 0]\n\
                   28: FitsRendered [3, 0, 0]\n\
                   29: SpanBounds [3, 0, 0]\n\
                   30: Select [3, 0, 0]\n\
                   31: Select [4, 0, 0]\n\
                   32: Select [3, 0, 0]\n\
                   33: Capture [4, 0, 0]\n\
                   34: Capture [2, 0, 0]\n\
                   35: Union [2, 2, 0]\n\
                   released: [0, 0, 0]";

/// Every node of the text program, the kinds it lacks after it (a filter of the relation, of the tree and of its edges, a
/// capture of the tree and of a node, a union of a node with itself), each run over three `DEFINES` rows and the small tree:
/// the rows each was fed, by its kind, and what the last reports once released.
fn fed_of_every_kind() -> Result<String, Failure> {
    let mut texts = TextResources::small_tree().rendering(vec![RenderOutcome::NotApplicable; 3]);
    let span = Span {
        start: Offset(0),
        end: Offset(1),
    };
    // The names the case program's two keyed selects look up, so every node reading them reads rows.
    let (alpha, beta) = (texts.mint(CellText("alpha")), texts.mint(CellText("beta")));
    let rows = defines_rows(&[(alpha, span), (beta, span), (alpha, span)]);
    let inputs = TableInputs::new(&[(DEFINES, &rows)])
        .with_roots((0..4).map(|at| node_id(FileId(0), Position(at))).collect());
    let mut steps = text_program();
    let every_row = |relation: Relation| Step {
        operation: Operation::Select {
            from: StepInput::Relation(relation),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        inputs: vec![PlanInput::Relation(relation)],
    };
    let capture = |from: PlanInput, path: u32| Step {
        operation: Operation::Capture {
            from: input_of(from),
            path: ColumnIndex(path),
            pattern: CapturePattern("{x}".into()),
        },
        inputs: vec![from],
    };
    steps.extend([
        every_row(DEFINES),
        every_row(Relation::SyntaxTree),
        every_row(Relation::SyntaxChild),
        capture(PlanInput::Relation(Relation::SyntaxTree), 2),
        capture(node(0), 3),
        over_nodes(
            Operation::Union {
                left: input_of(node(0)),
                right: input_of(node(0)),
            },
            &[0, 0],
        ),
    ]);
    let mut program = LogicalProgram::new();
    for step in &steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    let nodes: Vec<PlanNodeId> = (0..u32::try_from(steps.len()).unwrap_or(u32::MAX))
        .map(PlanNodeId)
        .collect();
    for (rule, node) in (0..).map(RuleOrdinal).zip(&nodes) {
        program.emit(rule, *node);
    }
    let plan = planned(&program)?;
    let mut buffers = FileBuffers::default();
    let mut lines = Vec::with_capacity(nodes.len() + 1);
    for (node, step) in nodes.iter().zip(&steps) {
        // A node's fault is its own answer: what is checked is what its kernels read to make it.
        let answer = ran(&plan, *node, &mut buffers, &inputs, &mut texts);
        let fed = buffers.fed(*node).0.map(|rows| rows.0);
        let fault = answer
            .err()
            .map_or_else(String::new, |fault| format!(" {fault:?}"));
        lines.push(format!(
            "{}: {:?} {fed:?}{fault}",
            node.0,
            kind_of(&step.operation)
        ));
    }
    let last = nodes.last().copied().unwrap_or(PlanNodeId(0));
    buffers.release(last);
    lines.push(format!(
        "released: {:?}",
        buffers.fed(last).0.map(|rows| rows.0)
    ));
    Ok(lines.join("\n"))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::EveryKind => fed_of_every_kind().map(Cow::Owned),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
