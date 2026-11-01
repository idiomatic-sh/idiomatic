//! The `visit` service: free functions named by verb that decide what a visit of a file must
//! build, with no IO and no ports.

use ryo_language::domain::relations::types::Relation;

use super::errors::VisitError;
use super::types::{
    BorrowedBytes, EdgeInput, LineCount, Noting, RoleCompared, RoleNamed, RoleRead, Wants,
};
use crate::domain::lowering::constants::EDGE;
use crate::domain::lowering::types::ElementPart;
use ryo_values::domain::plan_parts::service::each_column;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan};

/// The relations the plans read, each once, in the order they are first read.
///
/// # Errors
///
/// `UnknownRelation` when a plan reads a relation the catalog does not list.
pub(crate) fn wanted(plans: &[&Plan], catalog: &[Relation]) -> Result<Vec<Relation>, VisitError> {
    let mut wanted: Vec<Relation> = Vec::new();
    for plan in plans {
        for step in &plan.steps {
            for input in named(&inputs_of(&step.operation)) {
                let Input::Relation(relation) = *input else {
                    continue;
                };
                if !catalog.contains(&relation) {
                    return Err(VisitError::UnknownRelation { relation });
                }
                if !wanted.contains(&relation) {
                    wanted.push(relation);
                }
            }
        }
    }
    Ok(wanted)
}

/// The relations among those wanted that a visit of a file walks. The elements of the file's tree and its edges are
/// not among them: the table of the file says them, and the operators read them there.
#[must_use]
pub(crate) fn walked(wanted: &[Relation]) -> Vec<Relation> {
    wanted
        .iter()
        .copied()
        .filter(|relation| !matches!(*relation, Relation::SyntaxChild | Relation::SyntaxTree))
        .collect()
}

/// How many lines the bytes hold: each line feed ends one, and bytes after the last line feed are one more.
#[must_use]
pub(crate) fn lines_in(bytes: BorrowedBytes<'_>) -> LineCount {
    let bytes = bytes.0;
    let breaks = memchr::memchr_iter(b'\n', bytes).count();
    LineCount(
        i64::try_from(breaks).unwrap_or(i64::MAX)
            + i64::from(bytes.last().is_some_and(|last| *last != b'\n')),
    )
}

/// The place of the `field` column among the columns of `syntax-child`.
fn role_column() -> ColumnIndex {
    ColumnIndex(
        u32::try_from(
            EDGE.iter()
                .position(|each| *each == ElementPart::Role)
                .unwrap_or(usize::MAX),
        )
        .unwrap_or(u32::MAX),
    )
}

/// Whether an input of a step is the edges of the tree.
fn edges(input: Input) -> EdgeInput {
    if matches!(
        input,
        Input::Relation(Relation::SyntaxChild | Relation::SemanticChild)
    ) {
        EdgeInput::Edges
    } else {
        EdgeInput::Elsewhere
    }
}

/// Whether the columns named are all of them, or name the role.
fn names_role(columns: &[ColumnIndex]) -> RoleNamed {
    if columns.is_empty() || columns.contains(&role_column()) {
        RoleNamed::Named
    } else {
        RoleNamed::Unnamed
    }
}

/// Whether a comparison reads the role.
fn compares_role(comparisons: &[Comparison]) -> RoleCompared {
    let role = role_column();
    let mut compared = false;
    for each in comparisons {
        for operand in [&each.left, &each.right] {
            each_column(operand, &mut |column| compared |= column == role);
        }
    }
    if compared {
        RoleCompared::Compared
    } else {
        RoleCompared::Uncompared
    }
}

/// Whether a step reads the `field` column of the edges of the tree: the edges are an input of it and it may read the column.
fn reads_role(operation: &Operation) -> RoleRead {
    let reads = match operation {
        Operation::Select {
            from,
            comparisons,
            kept,
        } => {
            edges(*from) == EdgeInput::Edges
                && (names_role(kept) == RoleNamed::Named
                    || compares_role(comparisons) == RoleCompared::Compared)
        }
        Operation::Project { from, columns, .. } => {
            edges(*from) == EdgeInput::Edges && columns.contains(&role_column())
        }
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            output,
            filter,
            view,
        } => {
            (edges(*right) == EdgeInput::Edges
                && (view.contains(&role_column())
                    || right_on.contains(&role_column())
                    || compares_role(filter) == RoleCompared::Compared
                    || output
                        .iter()
                        .any(|each| each.side == Side::Right && each.column == role_column())))
                || (edges(*left) == EdgeInput::Edges
                    && (left_on.contains(&role_column())
                        || output.is_empty()
                        || output
                            .iter()
                            .any(|each| each.side == Side::Left && each.column == role_column())))
        }
        Operation::Antijoin {
            left,
            right,
            left_on,
            right_on,
        } => {
            edges(*left) == EdgeInput::Edges
                || (edges(*right) == EdgeInput::Edges && right_on.contains(&role_column()))
                || (edges(*left) == EdgeInput::Edges && left_on.contains(&role_column()))
        }
        Operation::Axis {
            from,
            domain,
            domain_id,
            domain_filter,
            domain_view,
            ..
        } => {
            edges(*from) == EdgeInput::Edges
                || (edges(*domain) == EdgeInput::Edges
                    && (domain_view.contains(&role_column())
                        || *domain_id == role_column()
                        || compares_role(domain_filter) == RoleCompared::Compared))
        }
        Operation::SpanBetween { from, .. }
        | Operation::Replace { from, .. }
        | Operation::Extend { from, .. }
        | Operation::GroupCount { from, .. }
        | Operation::GroupDigest { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Words { from, .. } => edges(*from) == EdgeInput::Edges,
        Operation::OrderedMatch { left, right, .. }
        | Operation::Union { left, right }
        | Operation::Intersection { left, right }
        | Operation::Difference { left, right } => {
            edges(*left) == EdgeInput::Edges || edges(*right) == EdgeInput::Edges
        }
        Operation::OrderedDfs {
            from, edges: graph, ..
        } => edges(*from) == EdgeInput::Edges || edges(*graph) == EdgeInput::Edges,
        Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => false,
    };
    if reads {
        RoleRead::Reads
    } else {
        RoleRead::Skips
    }
}

/// Whether the plans read the role of an edge of the tree: only then is a language asked to give the elements their roles.
#[must_use]
pub(crate) fn roles_of(plans: &[&Plan]) -> RoleRead {
    if plans.iter().any(|plan| {
        plan.steps
            .iter()
            .any(|step| reads_role(&step.operation) == RoleRead::Reads)
    }) {
        RoleRead::Reads
    } else {
        RoleRead::Skips
    }
}

/// What the plans read of the files: the relations asked of the languages, those the files are walked for, whether notes are written
/// and whether elements are given their roles.
///
/// # Errors
///
/// `UnknownRelation` when a plan reads a relation the catalog does not list.
pub(crate) fn wants(plans: &[Plan], catalog: &[Relation]) -> Result<Wants, VisitError> {
    let refs: Vec<&Plan> = plans.iter().collect();
    let wanted = wanted(&refs, catalog)?;
    let walked = walked(&wanted);
    let noted = if wanted.contains(&Relation::SourceDependency)
        || wanted.contains(&Relation::AnalyzedSource)
    {
        Noting::Wanted
    } else {
        Noting::Unwanted
    };
    let roled = roles_of(&refs);
    Ok(Wants {
        wanted,
        walked,
        noted,
        roled,
    })
}
