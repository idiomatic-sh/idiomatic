//! The services of the `emptiness` part: the formula of each operation from its inputs' formulas, and of a relation's read.
//! The `match` names every operation: a new one does not compile without its formula. The executor decides a formula once
//! per file, before anything of the node runs; a formula never says a node has a row, only when it cannot.

use crate::domain::lowered::types::{Emptiness, RowValue};
use crate::domain::steps::types::TestOperation;
use crate::domain::steps::types::AnchorKinds;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::types::{Column, FactForm, NodeColumn, Relation};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::written_rows::types::RowsSource;

/// Every part holds.
fn all(parts: Vec<Emptiness>) -> Emptiness {
    Emptiness::All(parts.into())
}

/// The formula of an operation from the formulas of its inputs, in the operation's input order.
#[must_use]
pub(crate) fn derived(operation: &Operation, mut inputs: Vec<Emptiness>) -> Emptiness {
    let mut first = || {
        if inputs.is_empty() {
            Emptiness::Unknown
        } else {
            inputs.remove(0)
        }
    };
    match operation {
        // A row of the answer is a row of `from`.
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::Words { .. }
        | Operation::SpanBetween { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::GroupBound { .. }
        | Operation::GroupOrdinal { .. } => first(),
        // A row needs a row of each side.
        Operation::Join { .. }
        | Operation::AsOf { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Intersection { .. } => all(inputs),
        // A row of either side.
        Operation::Union { .. } => Emptiness::Any(inputs.into()),
        // A left row; the right side only takes rows away.
        Operation::Antijoin { .. } | Operation::Difference { .. } => first(),
        // A start, an edge and a row of the domain.
        Operation::Axis { .. } => all(inputs),
        // An edge.
        Operation::Closure { .. } | Operation::Follow { .. } | Operation::Cycles { .. } => first(),
        // A root: a root state with no lattice state is a refusal today, never no rows, so the lattice is not asked.
        Operation::OrderedDfs { .. } | Operation::Worklist { .. } => first(),
        Operation::Rows { source, .. } => match source {
            RowsSource::Written(rows) if rows.0.is_empty() => Emptiness::Any(Box::default()),
            RowsSource::Written(_) => all(Vec::new()),
            RowsSource::Parameter(_) => Emptiness::Unknown,
        },
        Operation::Hosted { .. } => Emptiness::Unknown,
    }
}

/// The formula of a relation read whole: a row of a filled relation; nothing is known of the node table before it is read.
#[must_use]
pub(crate) fn relation_read(relation: Relation) -> Emptiness {
    match fact_form(relation) {
        FactForm::NodeTable => Emptiness::Unknown,
        FactForm::Filled(_) => Emptiness::Row(relation, None),
    }
}

/// The formula of a relation read with a literal in a column: a node of the kind when the column is the node table's kind
/// and the vocabulary names it; a row holding the value when the relation is filled, or when the column is the node
/// table's field (a file that holds no child in that field has no such row); else that of the relation read whole.
#[must_use]
pub(crate) fn literal_read(
    relation: Relation,
    column: Column,
    value: CellWord,
    kinds: Option<AnchorKinds>,
) -> Emptiness {
    match (fact_form(relation), node_column(column), kinds) {
        (FactForm::NodeTable, Some(NodeColumn::Kind), Some(kinds)) => Emptiness::Kind(kinds),
        (FactForm::NodeTable, Some(NodeColumn::Field), _) | (FactForm::Filled(_), _, _) => {
            Emptiness::Row(relation, Some(RowValue { column, value }))
        }
        (FactForm::NodeTable, _, _) => Emptiness::Unknown,
    }
}

/// A copy of a formula: each branch of a rule holds its own.
#[must_use]
pub(crate) fn copied(formula: &Emptiness) -> Emptiness {
    let each = |parts: &[Emptiness]| parts.iter().map(copied).collect();
    match formula {
        Emptiness::Unknown => Emptiness::Unknown,
        Emptiness::Kind(kinds) => Emptiness::Kind(kinds.clone()),
        Emptiness::Row(relation, value) => Emptiness::Row(*relation, *value),
        Emptiness::Definition(definition) => Emptiness::Definition(*definition),
        Emptiness::Table(table) => Emptiness::Table(*table),
        Emptiness::Path(test) => Emptiness::Path(*test),
        Emptiness::Bytes(literal) => Emptiness::Bytes(*literal),
        Emptiness::All(parts) => Emptiness::All(each(parts)),
        Emptiness::Any(parts) => Emptiness::Any(each(parts)),
    }
}

/// Whether a test of the scanned file against a constant is one the needs ask of the path before anything is filled: a
/// test of a text, which the file's path answers alone (`programs::collections::instructions::comparing`'s `text`). A
/// comparison of cells, a set or a number is not.
#[must_use]
pub(crate) const fn path_test(test: TestOperation) -> bool {
    match test {
        TestOperation::Matches(_)
        | TestOperation::Glob
        | TestOperation::NotGlob
        | TestOperation::StartsWith
        | TestOperation::EndsWith => true,
        TestOperation::Equal
        | TestOperation::NotEqual
        | TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::In
        | TestOperation::NotIn
        | TestOperation::NumericAtLeast(_)
        | TestOperation::NaturalLess
        | TestOperation::Successor(_) => false,
    }
}
