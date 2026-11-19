//! Stable distinct projection after one or two ordered source filters.

use crate::domain::executable_program::types::{
    CheckedSourceNode, Draft, ProjectedSourceRows, SchemaTiming, SourceAbsence, SourceColumnMap,
    SourceViewBranch,
};
use ryo_operators::domain::physical_plan::types::PlanInput;
use ryo_operators::domain::tree_cells::service::native_form;
use ryo_relations::domain::relations::constants::{LOWERED, RAISED};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Operand, QueryLiteral, RepeatedRows, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;

/// An ordinary validated draft and its physical readers. Frontiers, observations,
/// refused schemas and cross-file acquisition cannot enter this equivalence.
fn node<'s, 'p>(
    input: PlanInput,
    drafts: &'s [Draft<'p>],
    inputs: &'s [Vec<PlanInput>],
) -> Option<CheckedSourceNode<'s, 'p>> {
    let PlanInput::Node(id) = input else {
        return None;
    };
    let Ok(at) = usize::try_from(id.0) else {
        return None;
    };
    let d = drafts.get(at)?;
    if d.scope != StepScope::PerFile
        || d.timing != SchemaTiming::Stated
        || d.schema.is_err()
        || d.schema != d.layout
    {
        return None;
    }
    Some(CheckedSourceNode {
        operation: d.operation,
        inputs: inputs.get(at)?.as_slice(),
    })
}

/// A source, with the map of an optional stable first-distinct projection.
fn source<'p>(
    input: PlanInput,
    drafts: &[Draft<'p>],
    inputs: &[Vec<PlanInput>],
) -> Option<SourceColumnMap<'p>> {
    if let PlanInput::Relation(relation) = input {
        return Some(SourceColumnMap {
            relation,
            projection: None,
        });
    }
    let CheckedSourceNode {
        operation:
            Operation::Project {
                columns,
                distinct: RepeatedRows::Dropped,
                ..
            },
        inputs: [PlanInput::Relation(relation)],
    } = node(input, drafts, inputs)?
    else {
        return None;
    };
    Some(SourceColumnMap {
        relation: *relation,
        projection: Some(columns),
    })
}

/// Translate a column through an optional projection; missing columns decline.
fn through(column: ColumnIndex, projection: Option<&[ColumnIndex]>) -> Option<ColumnIndex> {
    match projection {
        Some(columns) => columns.get(usize::from(column)).copied(),
        None => Some(column),
    }
}

/// A single total equality filter and the final output columns, both in the source schema.
fn branch<'p>(
    input: PlanInput,
    output: &[ColumnIndex],
    drafts: &[Draft<'p>],
    inputs: &[Vec<PlanInput>],
) -> Option<ProjectedSourceRows<'p>> {
    let filter = node(input, drafts, inputs)?;
    filtered(filter.operation, filter.inputs, output, drafts, inputs)
}

/// Translate an equality filter and its output columns through its checked input.
fn filtered<'p>(
    operation: &'p Operation,
    list: &[PlanInput],
    output: &[ColumnIndex],
    drafts: &[Draft<'p>],
    inputs: &[Vec<PlanInput>],
) -> Option<ProjectedSourceRows<'p>> {
    let Operation::Select {
        comparisons, kept, ..
    } = operation
    else {
        return None;
    };
    let [one] = list else {
        return None;
    };
    let [test] = comparisons.as_slice() else {
        return None;
    };
    let (Test::Equal, Operand::Column(col), Operand::Literal(value)) =
        (test.test, &test.left, &test.right)
    else {
        return None;
    };
    let SourceColumnMap {
        relation,
        projection,
    } = source(*one, drafts, inputs)?;
    // Native tree views have a separate acquisition and row-order contract.
    if native_form(relation).is_some() {
        return None;
    }
    let key = through(*col, projection)?;
    let kind = kinds(relation).get(usize::from(key))?;
    match (kind, value) {
        (CellKind::Number, QueryLiteral::Number(_)) | (CellKind::Symbol, QueryLiteral::Text(_)) => {
        }
        (CellKind::Flag, QueryLiteral::Text(text))
            if matches!(text.0.as_ref(), RAISED | LOWERED) => {}
        _ => return None,
    }
    let kept = (!kept.is_empty()).then_some(kept.as_slice());
    let columns = output
        .iter()
        .map(|column| {
            let origin = through(through(*column, kept)?, projection)?;
            kinds(relation).get(usize::from(origin)).map(|_| origin)
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ProjectedSourceRows {
        relation,
        columns,
        first: SourceViewBranch { column: key, value },
        second: None,
    })
}

/// Signature for a first-distinct result. Each branch scans in provider order and the union
/// preserves left-before-right first occurrences. An earlier distinct projection may only
/// remove rows with identical final columns and predicate values, after their first witness.
/// Every route acquires precisely the same source, so an unavailable provider still refuses.
fn candidate<'p>(
    operation: &'p Operation,
    shape: &[CellKind],
    extended: Option<ryo_relations::domain::relations::types::Relation>,
    list: &[PlanInput],
    drafts: &[Draft<'p>],
    inputs: &[Vec<PlanInput>],
) -> Option<ProjectedSourceRows<'p>> {
    if let Some(relation) = extended
        && matches!(
            operation,
            Operation::Select { .. } | Operation::Union { .. }
        )
    {
        let Ok(count) = u32::try_from(shape.len()) else {
            return None;
        };
        let identity = (0..count).map(ColumnIndex).collect::<Vec<_>>();
        match (operation, list) {
            (Operation::Select { kept, .. }, [one]) => {
                let projection = source(*one, drafts, inputs)?.projection?;
                if !kept.is_empty()
                    && !(0..projection.len())
                        .all(|n| u32::try_from(n).is_ok_and(|ix| kept.contains(&ColumnIndex(ix))))
                {
                    return None;
                }
                return filtered(operation, list, &identity, drafts, inputs)
                    .filter(|view| view.relation == relation);
            }
            (Operation::Union { .. }, [left, right]) => {
                let mut lhs = branch(*left, &identity, drafts, inputs)?;
                let rhs = branch(*right, &identity, drafts, inputs)?;
                if lhs.relation != relation
                    || lhs.relation != rhs.relation
                    || lhs.columns != rhs.columns
                {
                    return None;
                }
                lhs.second = Some(rhs.first);
                return Some(lhs);
            }
            _ => return None,
        }
    }
    let Operation::Project {
        columns,
        distinct: RepeatedRows::Dropped,
        ..
    } = operation
    else {
        return None;
    };
    let [one] = list else { return None };
    let (first, second) = match node(*one, drafts, inputs)? {
        CheckedSourceNode {
            operation: Operation::Select { .. },
            ..
        } => (*one, None),
        CheckedSourceNode {
            operation: Operation::Union { .. },
            inputs: [left, right],
        } => (*left, Some(*right)),
        _ => return None,
    };
    let mut result = branch(first, columns, drafts, inputs)?;
    if let Some(next) = second {
        let other = branch(next, columns, drafts, inputs)?;
        if other.relation != result.relation || other.columns != result.columns {
            return None;
        }
        result.second = Some(other.first);
    }
    Some(result)
}

/// Reduce only the certified branch and only for the corresponding typed source.
fn conditioned<'p>(
    mut view: ProjectedSourceRows<'p>,
    condition: Option<&SourceAbsence>,
) -> Option<ProjectedSourceRows<'p>> {
    let Some(proof) = condition else {
        return Some(view);
    };
    if proof.relation != view.relation {
        return Some(view);
    }
    let is_absent = |branch: SourceViewBranch<'_>| {
        branch.column == proof.column
            && matches!(branch.value, QueryLiteral::Text(value) if *value == proof.text)
    };
    if is_absent(view.first) {
        let other = view.second.take()?;
        if is_absent(other) {
            return None;
        }
        view.first = other;
    } else if view.second.is_some_and(is_absent) {
        view.second = None;
    }
    Some(view)
}

/// An ordered, schema-checked value program through the source family available to this plan.
pub(in crate::domain::executable_program) fn signature<'p>(
    operation: &'p Operation,
    list: &[PlanInput],
    drafts: &[Draft<'p>],
    state: &crate::domain::executable_program::types::program_inputs::value::ProgramInputs,
    shape: &Result<Vec<CellKind>, crate::domain::operating::errors::OperatingError>,
) -> Option<ProjectedSourceRows<'p>> {
    let Ok(kinds) = shape else {
        return None;
    };
    let view = candidate(operation, kinds, state.extended, list, drafts, &state.lists)?;
    conditioned(view, state.absence.as_ref())
}
