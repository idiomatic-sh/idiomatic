//! The cases of the `emptiness` part: the formula of each operation, one row per kind of operation, over inputs whose
//! formulas are a row of `Function`, of `Comment` and of `Scope` (as many as the operation reads, in its input order); and
//! the formula of a relation read whole and with a literal. Every expected text is what a run printed.

use super::service::{derived, literal_read, relation_read};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::fixtures::growth::growth_plan;
use crate::domain::steps::types::AnchorKinds;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::collections::relations::syntax_error::item::SyntaxErrorColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::types::{Column, KindBits, Relation};
use ryo_values::domain::plans::service::{inputs_of, kind_of, named};
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;

/// A formula as derived.
type Input = Emptiness;
/// The formula, said: its debug text.
type Value = Cow<'static, str>;
type Failure = ();

const fn row(
    name: &'static str,
    input: fn() -> Input,
    said: &'static str,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(Cow::Borrowed(said)),
    }
}

/// The formula of the first step of `kind` in its growth plan, its inputs' formulas being a row of `Function`, of `Comment`
/// and of `Scope`, in input order, as many as the operation reads.
fn of(kind: OperationKind) -> Emptiness {
    let plan = growth_plan(kind);
    let step = plan
        .steps
        .iter()
        .find(|step| kind_of(&step.operation) == kind);
    let Some(step) = step else {
        return Emptiness::Unknown;
    };
    let inputs = [Relation::Function, Relation::Comment, Relation::Scope];
    let read = named(&inputs_of(&step.operation)).len();
    derived(
        &step.operation,
        inputs.into_iter().take(read).map(relation_read).collect(),
    )
}

const KIND: Column = Column::SyntaxTree(SyntaxTreeColumn::Kind);
const ELEMENT: Column = Column::SyntaxError(SyntaxErrorColumn::Id);

/// The rows of this part's cases: one per kind of operation, then the reads of a relation.
const CASES: &[Case<Input, Value, Failure>] = &[
    row(
        "replace",
        || of(OperationKind::Replace),
        "Row(Function, None)",
    ),
    row(
        "ordered_match",
        || of(OperationKind::OrderedMatch),
        "All([Row(Function, None), Row(Comment, None)])",
    ),
    row(
        "as_of",
        || of(OperationKind::AsOf),
        "All([Row(Function, None), Row(Comment, None)])",
    ),
    row(
        "select",
        || of(OperationKind::Select),
        "Row(Function, None)",
    ),
    row(
        "project",
        || of(OperationKind::Project),
        "Row(Function, None)",
    ),
    row(
        "extend",
        || of(OperationKind::Extend),
        "Row(Function, None)",
    ),
    row(
        "join",
        || of(OperationKind::Join),
        "All([Row(Function, None), Row(Comment, None)])",
    ),
    row(
        "union",
        || of(OperationKind::Union),
        "Any([Row(Function, None), Row(Comment, None)])",
    ),
    row(
        "intersection",
        || of(OperationKind::Intersection),
        "All([Row(Function, None), Row(Comment, None)])",
    ),
    row(
        "difference",
        || of(OperationKind::Difference),
        "Row(Function, None)",
    ),
    row(
        "antijoin",
        || of(OperationKind::Antijoin),
        "Row(Function, None)",
    ),
    row(
        "group_count",
        || of(OperationKind::GroupCount),
        "Row(Function, None)",
    ),
    row(
        "group_bound",
        || of(OperationKind::GroupBound),
        "Row(Function, None)",
    ),
    row(
        "group_ordinal",
        || of(OperationKind::GroupOrdinal),
        "Row(Function, None)",
    ),
    row(
        "group_digest",
        || of(OperationKind::GroupDigest),
        "Row(Function, None)",
    ),
    row(
        "capture",
        || of(OperationKind::Capture),
        "Row(Function, None)",
    ),
    row("rank", || of(OperationKind::Rank), "Row(Function, None)"),
    row("words", || of(OperationKind::Words), "Row(Function, None)"),
    row(
        "axis",
        || of(OperationKind::Axis),
        "All([Row(Function, None), Row(Comment, None), Row(Scope, None)])",
    ),
    row(
        "closure",
        || of(OperationKind::Closure),
        "Row(Function, None)",
    ),
    row(
        "follow",
        || of(OperationKind::Follow),
        "Row(Function, None)",
    ),
    row(
        "cycles",
        || of(OperationKind::Cycles),
        "Row(Function, None)",
    ),
    row("rows", || of(OperationKind::Rows), "All([])"),
    row("hosted", || of(OperationKind::Hosted), "Unknown"),
    row(
        "span_between",
        || of(OperationKind::SpanBetween),
        "Row(Function, None)",
    ),
    row(
        "source_matches",
        || of(OperationKind::SourceMatches),
        "Row(Function, None)",
    ),
    row(
        "render_syntax",
        || of(OperationKind::RenderSyntax),
        "Row(Function, None)",
    ),
    row(
        "fits_rendered",
        || of(OperationKind::FitsRendered),
        "Row(Function, None)",
    ),
    row(
        "span_bounds",
        || of(OperationKind::SpanBounds),
        "Row(Function, None)",
    ),
    row(
        "ordered_dfs",
        || of(OperationKind::OrderedDfs),
        "Row(Function, None)",
    ),
    row(
        "worklist",
        || of(OperationKind::Worklist),
        "Row(Function, None)",
    ),
    row(
        "the_node_table_read_whole_says_nothing",
        || relation_read(Relation::SyntaxTree),
        "Unknown",
    ),
    row(
        "a_filled_relation_read_whole_needs_a_row_of_it",
        || relation_read(Relation::SyntaxError),
        "Row(SyntaxError, None)",
    ),
    row(
        "a_kind_the_vocabulary_does_not_name_says_nothing",
        || literal_read(Relation::SyntaxTree, KIND, CellWord(3), None),
        "Unknown",
    ),
    row(
        "a_kind_the_vocabulary_names_needs_a_node_of_it",
        || {
            literal_read(
                Relation::SyntaxTree,
                KIND,
                CellWord(3),
                Some(AnchorKinds([KindBits(4)].into())),
            )
        },
        "Kind(AnchorKinds([KindBits(4)]))",
    ),
    row(
        "a_filled_relation_read_with_a_literal_needs_a_row_holding_it",
        || literal_read(Relation::SyntaxError, ELEMENT, CellWord(3), None),
        "Row(SyntaxError, Some(RowValue { column: SyntaxError(Id), value: CellWord(3) }))",
    ),
];

fn subject(formula: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(format!("{formula:?}")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
