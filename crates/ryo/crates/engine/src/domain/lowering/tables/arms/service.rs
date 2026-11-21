//! The services of the `arms` part: a rule's steps flattened, each once, in step order. A step's branches are taken by its one
//! reader; a step with several readers across the pack, the right side of an absence, the rows of a fill and an input past
//! a bound are each one definition, and their reader holds one use. The `match` over `Operation` names every operation.

use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::definitions::service::kind_bits;
use crate::domain::lowering::definitions::service::text_word;
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::{derived, literal_read};
use crate::domain::lowering::flattening::service::picked;
use crate::domain::lowering::flattening::types::RowColumn;
use crate::domain::lowering::nodes::service::digest_of;
use crate::domain::lowering::nodes::types::StepReach;
use crate::domain::lowering::tables::calls::service::call_arm;
use crate::domain::lowering::tables::closures::service::closure_arm;
use crate::domain::lowering::tables::fills::service::{as_of_arm, group_count_arm, group_rows_arm};
use crate::domain::lowering::tables::inputs::service::{all_selected, input, rule, term};
use crate::domain::lowering::tables::joins::service::{antijoin_arm, axis_arm, join_arm};
use crate::domain::lowering::tables::service::test_of;
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening};
use crate::domain::steps::types::ConstantText;
use crate::domain::steps::types::{AnchorKinds, TestOperation};
use ryo_relations::domain::relations::service::columns;
use ryo_values::domain::plan_parts::types::{Input, Operand, PlanComparison, QueryLiteral, StepId};
use ryo_values::domain::plans::types::Operation;

/// The emptiness of a selection: of a relation read with a literal in a column when that is what it is, else its input's.
fn selection_emptiness(
    state: &RuleFlattening<'_>,
    drafts: &Drafts,
    from: Input,
    comparisons: &[PlanComparison],
    read: Emptiness,
) -> Emptiness {
    let Input::Relation(relation) = from else {
        return read;
    };
    let listed = columns(relation);
    for comparison in comparisons {
        let (Operand::Column(column), Operand::Literal(QueryLiteral::Text(written))) =
            (&comparison.left, &comparison.right)
        else {
            continue;
        };
        let (Some(number), Some(column), Some(TestOperation::Equal)) = (
            drafts.text_numbers.get(written),
            listed.get(usize::from(*column)),
            test_of(comparison.test),
        ) else {
            continue;
        };
        let kinds = kind_bits(state.kinds, &ConstantText(written.0.clone()))
            .map(|bits| AnchorKinds([bits].into()));
        return literal_read(relation, *column, text_word(*number), kinds);
    }
    read
}

/// The flattening of one step. None for an operation the lowering does not evaluate: the rule was refused before it came here.
fn arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    match operation {
        Operation::Select {
            from,
            comparisons,
            kept,
        } => {
            let mut read = input(state, drafts, *from, InputWant::Inline)?;
            read.branches = all_selected(state, drafts, read.branches, comparisons)?;
            if !kept.is_empty() {
                for branch in &mut read.branches {
                    branch.row = picked(
                        &branch.row,
                        kept.iter().map(|column| RowColumn(usize::from(*column))),
                    )?;
                }
            }
            read.emptiness = selection_emptiness(state, drafts, *from, comparisons, read.emptiness);
            Some(read)
        }
        Operation::Project { from, columns, .. } => {
            let mut read = input(state, drafts, *from, InputWant::Inline)?;
            for branch in &mut read.branches {
                branch.row = picked(
                    &branch.row,
                    columns.iter().map(|column| RowColumn(usize::from(*column))),
                )?;
            }
            Some(read)
        }
        Operation::Extend { from, values } => {
            let mut read = input(state, drafts, *from, InputWant::Inline)?;
            for branch in &mut read.branches {
                for value in values {
                    let added = term(state, drafts, branch, value)?;
                    branch.row.push(added);
                }
            }
            Some(read)
        }
        Operation::Join { .. } => join_arm(state, drafts, step, operation),
        Operation::Union { left, right } => {
            let mut lefts = input(state, drafts, *left, InputWant::Inline)?;
            let rights = input(state, drafts, *right, InputWant::Inline)?;
            lefts.branches.extend(rights.branches);
            lefts.keyless.extend(rights.keyless);
            lefts.emptiness = derived(operation, vec![lefts.emptiness, rights.emptiness]);
            Some(lefts)
        }
        Operation::Antijoin { .. } => antijoin_arm(state, drafts, operation),
        Operation::GroupCount { .. } => group_count_arm(state, drafts, step, operation),
        Operation::GroupBound { .. } | Operation::GroupOrdinal { .. } => {
            group_rows_arm(state, drafts, step, operation)
        }
        Operation::AsOf { .. } => as_of_arm(state, drafts, step, operation),
        Operation::Axis { .. } => axis_arm(state, drafts, operation),
        Operation::Closure { .. } => closure_arm(state, drafts, operation),
        Operation::Replace { .. }
        | Operation::SpanBounds { .. }
        | Operation::SpanBetween { .. } => call_arm(state, drafts, operation),
        // What is left to lower, each by name: `tables::service::shaped` refuses a rule that holds one.
        Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Follow { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. } => None,
    }
}

/// The steps of the rule that are flattened: those its emit reaches without passing a step that already has a definition.
/// Each is visited once.
fn wanted(state: &RuleFlattening<'_>, drafts: &Drafts) -> Option<Vec<StepReach>> {
    let rule = rule(state)?;
    let plan = rule.rule.plan;
    let mut wanted = vec![StepReach::Unreached; plan.steps.len()];
    let mut waiting = vec![plan.emit.step];
    while let Some(step) = waiting.pop() {
        let place = usize::from(step);
        if wanted.get(place) != Some(&StepReach::Unreached)
            || drafts.defined.contains_key(&digest_of(rule, step))
        {
            continue;
        }
        *wanted.get_mut(place)? = StepReach::Reached;
        let inputs =
            ryo_values::domain::plans::service::inputs_of(&plan.steps.get(place)?.operation);
        for read in ryo_values::domain::plans::service::named(&inputs) {
            if let Input::Step(read) = read {
                waiting.push(*read);
            }
        }
    }
    Some(wanted)
}

/// The rows a rule emits, flattened: every step once, in step order. The step named is one that could not be flattened.
pub(crate) fn rule_flattened(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
) -> Result<Flattened, StepId> {
    let emitted = rule(state).map_or(StepId(0), |rule| rule.rule.plan.emit.step);
    let wanted = wanted(state, drafts).ok_or(emitted)?;
    let plan = rule(state).ok_or(emitted)?.rule.plan;
    state.flat = plan.steps.iter().map(|_| None).collect();
    for (place, step) in plan.steps.iter().enumerate() {
        if wanted.get(place) != Some(&StepReach::Reached) {
            continue;
        }
        let id = StepId(u32::try_from(place).unwrap_or(u32::MAX));
        let flattened = arm(state, drafts, id, &step.operation).ok_or(id)?;
        *state.flat.get_mut(place).ok_or(id)? = Some(flattened);
    }
    input(state, drafts, Input::Step(emitted), InputWant::Inline).ok_or(emitted)
}
