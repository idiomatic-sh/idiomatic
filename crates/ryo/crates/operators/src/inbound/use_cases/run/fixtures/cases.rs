//! The rows and steps the run use case's cases are made of: the relation they read, its rows, and the selects and comparisons
//! over it; and the probes of the tree a rewrite case runs.
use super::{TableInputs, TextResources, ran};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::ProbeIndex;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, PlanInputList, PlanInputs, Resources,
};
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    FileId, NodeId, Offset, Position, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the use cases read: its refinement (column 2) and name (column 3) are texts.
pub(crate) const DEFINES: Relation = Relation::Defines;

/// The rows of `DEFINES`, one per (refinement, name) pair, each a node of file 0, with `texts` the file's symbols.
pub(crate) fn defines(pairs: &[(&str, &str)], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(pairs.len() * kinds(DEFINES).len());
    for (at, (refinement, name)) in pairs.iter().enumerate() {
        let node = Value::Node(node_id(
            FileId(0),
            Position(u32::try_from(at).unwrap_or(u32::MAX)),
        ));
        cells.extend([
            node,
            texts.symbol("kind"),
            texts.symbol(refinement),
            texts.symbol(name),
            node,
            Value::File(FileId(0)),
            Value::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
        ]);
    }
    Table {
        kinds: kinds(DEFINES).to_vec(),
        cells,
        count: RowCount(pairs.len()),
    }
}

/// The rows of `DEFINES`, one for each of `nodes`, each named `name`, the node its own definition, in file 0 over no bytes.
pub(crate) fn defines_of_nodes(nodes: &[NodeId], name: Value) -> Table {
    let span = Value::Span(Span {
        start: Offset(0),
        end: Offset(0),
    });
    Table {
        kinds: kinds(DEFINES).to_vec(),
        cells: nodes
            .iter()
            .flat_map(|node| {
                [
                    Value::Node(*node),
                    name,
                    name,
                    name,
                    Value::Node(*node),
                    Value::File(FileId(0)),
                    span,
                ]
            })
            .collect(),
        count: RowCount(nodes.len()),
    }
}

/// A comparison of two operands.
pub(crate) fn compared(test: Test, left: Operand, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left,
        right,
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// Whether a column's text matches the pattern its own text is: a regex made from the row, which fails on a text that is no
/// pattern.
pub(crate) fn self_matching(column: u32) -> PlanComparison {
    compared(
        Test::Matches,
        Operand::Column(ColumnIndex(column)),
        Operand::Column(ColumnIndex(column)),
    )
}

/// Whether the name is `text`.
pub(crate) fn named(text: &str) -> PlanComparison {
    compared(
        Test::Equal,
        Operand::Column(ColumnIndex(3)),
        Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
    )
}

/// A select of every column of what `from` reads, kept where `comparisons` hold.
pub(crate) fn select(from: PlanInput, comparisons: Vec<PlanComparison>) -> Step {
    Step {
        operation: Operation::Select {
            from: input_of(from),
            comparisons,
            kept: Vec::new(),
        },
        inputs: vec![from],
    }
}

/// The plan's input as the operation names it.
pub(crate) fn input_of(from: PlanInput) -> StepInput {
    match from {
        PlanInput::Node(PlanNodeId(at)) => {
            StepInput::Step(ryo_values::domain::plan_parts::types::StepId(at))
        }
        PlanInput::Relation(relation) => StepInput::Relation(relation),
        PlanInput::Observation(place) => StepInput::BoundObservation(place.input),
        PlanInput::Frontier(place) => {
            StepInput::Step(ryo_values::domain::plan_parts::types::StepId(
                u32::try_from(place.input.0).unwrap_or(u32::MAX),
            ))
        }
    }
}

/// The rows each node of `emits` answers, each the emit of a rule of its own, the program of `steps` planned with `rewrites`
/// alone and run over `inputs` and `resources`: what a program answers with and without a rewrite.
///
/// # Errors
///
/// The fault the planner or the runner answers.
pub(crate) fn rewritten(
    steps: &[Step],
    emits: &[PlanNodeId],
    rewrites: &[&dyn RewritesItem],
    inputs: &dyn PlanInputs,
    resources: &mut dyn Resources,
) -> Result<Vec<Table>, PortableOperatorFault> {
    let plan = planned_with(steps, emits, rewrites)?;
    let mut buffers = FileBuffers::default();
    emits
        .iter()
        .map(|node| {
            let node = *node;
            ran(&plan, node, &mut buffers, inputs, resources)?;
            match buffers.rows(node) {
                NodeRows::Materialized(table) => Ok(table.clone()),
                NodeRows::Absent => Err(PortableOperatorFault::NotRun),
            }
        })
        .collect()
}

/// Whether the program of `steps` planned with every rewrite of `rewrites` probes the tree in each of its probes, having
/// some, and `every`, what it answered, is `answer`: rows, not all empty, or that fault.
///
/// # Errors
///
/// The planner's shape fault.
pub(crate) fn walks_and_answers(
    steps: &[Step],
    emits: &[PlanNodeId],
    rewrites: &[&dyn RewritesItem],
    every: &Result<Vec<Table>, PortableOperatorFault>,
    answer: &TreeAnswer,
) -> Result<bool, PortableOperatorFault> {
    let plan = planned_with(steps, emits, rewrites)?;
    let probes: Vec<&Lowering> = plan
        .nodes
        .iter()
        .map(|node| &node.lowering)
        .filter(|lowering| matches!(lowering, Lowering::Probe { .. }))
        .collect();
    let walks = !probes.is_empty()
        && probes.iter().all(|lowering| {
            matches!(
                lowering,
                Lowering::Probe {
                    index: ProbeIndex::Tree,
                    ..
                }
            )
        });
    let answered = match every {
        Ok(tables) => tables
            .iter()
            .any(|table| table.count.0 > 0)
            .then_some(TreeAnswer::Rows),
        Err(fault) => Some(TreeAnswer::Fault(fault.clone())),
    };
    Ok(walks && answered.as_ref() == Some(answer))
}

/// The plan of the program of `steps`, each node of `emits` the emit of a rule of its own, with `rewrites` alone.
///
/// # Errors
///
/// The planner's shape fault.
pub(crate) fn planned_with(
    steps: &[Step],
    emits: &[PlanNodeId],
    rewrites: &[&dyn RewritesItem],
) -> Result<PhysicalPlan, PortableOperatorFault> {
    let mut program = LogicalProgram::new();
    for step in steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    for (rule, node) in (0..).map(RuleOrdinal).zip(emits) {
        program.emit(rule, *node);
    }
    crate::domain::physical_plan::service::plan_with(&program, rewrites)
}

/// One program a rewrite case runs: its steps, the nodes it emits, and what it reads.
pub(crate) struct TreeRun {
    /// The steps.
    pub(crate) steps: Vec<Step>,
    /// The nodes each a rule's emit.
    pub(crate) emits: Vec<PlanNodeId>,
    /// The rows it reads.
    pub(crate) inputs: TableInputs,
    /// The file it reads them in.
    pub(crate) resources: TextResources,
    /// What it answers with every rewrite.
    pub(crate) answer: TreeAnswer,
}

/// What a tree run answers: rows, some of them, or a fault.
#[derive(Debug, PartialEq)]
pub(crate) enum TreeAnswer {
    /// Its emitted nodes' rows, not all empty.
    Rows,
    /// This fault.
    Fault(PortableOperatorFault),
}

/// The join of `DEFINES`' column `left_on` with a tree relation's rows on `right_on`, answering `output`, its right rows kept
/// where `filter` holds.
fn tree_join(
    left_on: u32,
    right: (Relation, u32),
    output: &[(Side, u32)],
    filter: Vec<PlanComparison>,
) -> Step {
    Step {
        operation: Operation::Join {
            left: StepInput::Relation(DEFINES),
            right: StepInput::Relation(right.0),
            left_on: vec![ColumnIndex(left_on)],
            right_on: vec![ColumnIndex(right.1)],
            output: output
                .iter()
                .map(|(side, column)| JoinColumn {
                    side: *side,
                    column: ColumnIndex(*column),
                })
                .collect(),
            filter,
            view: Vec::new(),
            reach: None,
        },
        inputs: vec![PlanInput::Relation(DEFINES), PlanInput::Relation(right.0)],
    }
}

/// The antijoin of `left`'s column `left_on` with a tree relation's rows on `right_on`.
fn tree_antijoin(left: PlanInput, left_on: u32, right: (Relation, u32)) -> Step {
    Step {
        operation: Operation::Antijoin {
            left: input_of(left),
            right: StepInput::Relation(right.0),
            left_on: vec![ColumnIndex(left_on)],
            right_on: vec![ColumnIndex(right.1)],
        },
        inputs: vec![left, PlanInput::Relation(right.0)],
    }
}

/// The probes of the tree a rewrite must keep the rows of: inner probes and antijoins of the nested tree's edges by parent
/// and by child and of its elements by id, over `DEFINES` rows of a node of three children, the root, that node again, a node
/// of another file, a node past the tree and a token; an inner probe answering the same row for each child, one keyed on a
/// column of no node, and one with a right filter that cannot fail. Then rows narrower than their key against the nested
/// tree's edges, which fault, and against a tree of only its root, which keep every row unread.
pub(crate) fn tree_probe_runs() -> Vec<TreeRun> {
    let (edges, elements) = (Relation::SyntaxChild, Relation::SyntaxTree);
    let nested = TextResources::nested_tree();
    let left = defines_of_nodes(
        &[
            node_id(FileId(0), Position(1)),
            node_id(FileId(0), Position(0)),
            node_id(FileId(0), Position(1)),
            node_id(FileId(1), Position(1)),
            node_id(FileId(0), Position(9)),
            node_id(FileId(0), Position(2)),
        ],
        nested.symbol("word"),
    );
    let reading = |nodes: u32| {
        TableInputs::new(&[(DEFINES, &left)]).with_roots(
            (0..nodes)
                .map(|at| node_id(FileId(0), Position(at)))
                .collect(),
        )
    };
    // The edges of a parent's first child: an equality of the role with a written text, which cannot fail, and which drops
    // every other edge a walk finds.
    let heads = compared(
        Test::Equal,
        Operand::Column(ColumnIndex(3)),
        Operand::Literal(QueryLiteral::Text(LiteralText("head".into()))),
    );
    let pair = [(Side::Left, 0), (Side::Right, 1)];
    let probes = vec![
        tree_join(0, (edges, 0), &pair, Vec::new()),
        tree_join(
            0,
            (edges, 1),
            &[(Side::Left, 0), (Side::Right, 0)],
            Vec::new(),
        ),
        tree_join(0, (elements, 0), &pair, Vec::new()),
        tree_join(0, (edges, 0), &[(Side::Left, 0)], Vec::new()),
        tree_join(3, (edges, 0), &pair, Vec::new()),
        tree_join(0, (edges, 0), &pair, vec![heads]),
        tree_antijoin(PlanInput::Relation(DEFINES), 0, (edges, 0)),
        tree_antijoin(PlanInput::Relation(DEFINES), 0, (edges, 1)),
        tree_antijoin(PlanInput::Relation(DEFINES), 0, (elements, 0)),
    ];
    let emits = (0..u32::try_from(probes.len()).unwrap_or(u32::MAX))
        .map(PlanNodeId)
        .collect();
    // Rows of one column, keyed on their sixth.
    let narrow = || {
        vec![
            Step {
                operation: Operation::Project {
                    from: StepInput::Relation(DEFINES),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
                inputs: vec![PlanInput::Relation(DEFINES)],
            },
            tree_antijoin(PlanInput::Node(PlanNodeId(0)), 5, (edges, 0)),
        ]
    };
    vec![
        TreeRun {
            steps: probes,
            emits,
            inputs: reading(6),
            resources: nested,
            answer: TreeAnswer::Rows,
        },
        TreeRun {
            steps: narrow(),
            emits: vec![PlanNodeId(1)],
            inputs: reading(6),
            resources: TextResources::nested_tree(),
            answer: TreeAnswer::Fault(PortableOperatorFault::NoSuchColumn {
                index: ColumnIndex(5),
            }),
        },
        TreeRun {
            steps: narrow(),
            emits: vec![PlanNodeId(1)],
            inputs: reading(1),
            resources: TextResources::root_only(),
            answer: TreeAnswer::Rows,
        },
    ]
}
