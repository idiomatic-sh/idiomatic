//! Test support of the `activation` concept: a case's program, built from real operations, its drafts and one file's
//! triggers, and each node's activity in that file.
use super::service::{activity, plan_of, trigger_kinds};
use super::types::{
    ActivationLeaves, Activities, FactAnswer, NodeActivity, TriggerPresence, Triggers,
};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::{Draft, SchemaTiming};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// One node of a case's program: its operation, what it reads, and where it runs.
pub(super) type Node = (Operation, Vec<PlanInput>, StepScope);

/// The drafts of `nodes`, each as stated, of no column.
pub(super) fn drafts(nodes: &[Node]) -> Vec<Draft<'_>> {
    nodes
        .iter()
        .map(|(operation, _, scope)| Draft {
            operation,
            rule: RuleIndex(0),
            scope: *scope,
            schema: Ok(Vec::new()),
            layout: Ok(Vec::new()),
            timing: SchemaTiming::Stated,
        })
        .collect()
}

/// The fact relation the programs read.
pub(super) const FACT: Relation = Relation::Function;
/// The root kind the programs' tree selects name.
pub(super) const CLASS: &str = "class_definition";

/// A program and one file's triggers.
#[derive(Debug)]
pub(super) struct Program {
    /// Its nodes, in node order.
    pub(super) nodes: Vec<Node>,
    /// Each root kind's text, with the nodes that start from it.
    pub(super) roots: Vec<(&'static str, Vec<u32>)>,
    /// The root kinds the file holds.
    pub(super) held: Vec<&'static str>,
    /// The fact relations the file has no row of.
    pub(super) empty: Vec<Relation>,
    /// The fact relations whose read the file refuses.
    pub(super) refused: Vec<Relation>,
}

/// A comparison of column `column` with `right`.
fn compared(test: Test, column: u32, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right,
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// A written text.
fn text(written: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(written.into())))
}

/// A select of the tree's nodes of kind `CLASS`, then by `suffix`: a root of that kind.
pub(super) fn root(suffix: Vec<PlanComparison>) -> Node {
    let mut comparisons = vec![compared(Test::Equal, 1, text(CLASS))];
    comparisons.extend(suffix);
    (
        Operation::Select {
            from: StepInput::Relation(Relation::SyntaxTree),
            comparisons,
            kept: Vec::new(),
        },
        vec![PlanInput::Relation(Relation::SyntaxTree)],
        StepScope::PerFile,
    )
}

/// A select of the fact relation's rows by `comparisons`.
pub(super) fn fact_select(comparisons: Vec<PlanComparison>) -> Node {
    (
        Operation::Select {
            from: StepInput::Relation(FACT),
            comparisons,
            kept: Vec::new(),
        },
        vec![PlanInput::Relation(FACT)],
        StepScope::PerFile,
    )
}

/// What a binary step over `left` and `right` reads.
fn pair(left: PlanInput, right: PlanInput) -> (StepInput, StepInput, Vec<PlanInput>) {
    let step = |input: PlanInput| match input {
        PlanInput::Node(node) => StepInput::Step(StepId(node.0)),
        PlanInput::Relation(relation) => StepInput::Relation(relation),
        PlanInput::Frontier(_) | PlanInput::Observation(_) => StepInput::Relation(FACT),
    };
    (step(left), step(right), vec![left, right])
}

/// A join of `left` with `right` on their first column, its right rows kept by `filter`.
pub(super) fn join(left: PlanInput, right: PlanInput, filter: Vec<PlanComparison>) -> Node {
    let (left_step, right_step, inputs) = pair(left, right);
    (
        Operation::Join {
            left: left_step,
            right: right_step,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            }],
            filter,
            view: Vec::new(),
            reach: None,
        },
        inputs,
        StepScope::PerFile,
    )
}

/// The rows of `left` with no row of `right` on their first column.
pub(super) fn antijoin(left: PlanInput, right: PlanInput) -> Node {
    let (left_step, right_step, inputs) = pair(left, right);
    (
        Operation::Antijoin {
            left: left_step,
            right: right_step,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
        inputs,
        StepScope::PerFile,
    )
}

/// The rows of `left` and of `right`.
pub(super) fn union(left: PlanInput, right: PlanInput) -> Node {
    let (left_step, right_step, inputs) = pair(left, right);
    (
        Operation::Union {
            left: left_step,
            right: right_step,
        },
        inputs,
        StepScope::PerFile,
    )
}

/// The first column of what `from` reads, run in `scope`.
pub(super) fn project(from: PlanInput, scope: StepScope) -> Node {
    let (step, _, _) = pair(from, from);
    (
        Operation::Project {
            from: step,
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
        vec![from],
        scope,
    )
}

/// The input reading node `at`.
pub(super) fn node(at: u32) -> PlanInput {
    PlanInput::Node(PlanNodeId(at))
}

/// A matching of column 1 against a regular expression: written, or read from column 2.
pub(super) fn matches(written: bool) -> PlanComparison {
    let right = if written {
        text("[a-z]+")
    } else {
        Operand::Column(ColumnIndex(2))
    };
    compared(Test::Matches, 1, right)
}

/// Each node's activity in the file `program` describes, asked in node order (`A` active, `D` dormant), then the reads made.
pub(super) fn activity_of(program: &Program) -> Result<Cow<'static, str>, OperatingError> {
    let lists: Vec<Vec<PlanInput>> = program.nodes.iter().map(|node| node.1.clone()).collect();
    let roots = program
        .roots
        .iter()
        .map(|(kind, nodes)| {
            (
                LiteralText((*kind).into()),
                nodes.iter().copied().map(PlanNodeId).collect(),
            )
        })
        .collect();
    let plan = plan_of(&drafts(&program.nodes), &lists, &roots);
    let triggers = Triggers {
        kinds: trigger_kinds(&plan)
            .iter()
            .map(|kind| {
                if program.held.contains(&&*kind.0) {
                    TriggerPresence::Present
                } else {
                    TriggerPresence::Absent
                }
            })
            .collect(),
    };
    let mut file = FakeFile {
        program,
        reads: Vec::new(),
    };
    let mut states = Activities::default();
    let mut answered = String::new();
    for at in 0..program.nodes.len() {
        let node = PlanNodeId(u32::try_from(at).unwrap_or(u32::MAX));
        answered.push(
            match activity(&plan, &triggers, &mut file, &mut states, node)? {
                NodeActivity::Active => 'A',
                NodeActivity::Dormant => 'D',
            },
        );
    }
    Ok(Cow::Owned(format!("{answered}; reads {:?}", file.reads)))
}

/// A file a case describes, answering activation's reads from its program's empty and refused relations, and keeping each read.
struct FakeFile<'p> {
    /// The program and the file it describes.
    program: &'p Program,
    /// Each relation read, in order.
    reads: Vec<Relation>,
}

impl ActivationLeaves for FakeFile<'_> {
    fn rows(&mut self, relation: Relation) -> FactAnswer {
        self.reads.push(relation);
        if self.program.refused.contains(&relation) {
            FactAnswer::Refused
        } else if self.program.empty.contains(&relation) {
            FactAnswer::NoRow
        } else {
            FactAnswer::Rows
        }
    }
}
