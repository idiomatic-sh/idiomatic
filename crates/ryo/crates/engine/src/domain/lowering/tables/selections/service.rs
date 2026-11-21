//! The services of the `selections` part: a branch with every comparison of a selection added, each over the branch's own
//! row, and what a file must hold for the selection to have a row. A comparison that holds a node's kind or field to a set
//! of literals gives one branch for each literal (`sets`); every other comparison is a pin or a test of the branch.

use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::definitions::service::{kind_bits, regex, text_word};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::literal_read;
use crate::domain::lowering::flattening::service::compared;
use crate::domain::lowering::flattening::types::{FlatBranch, FlatTerm};
use crate::domain::lowering::nodes::service::input_shape;
use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::tables::inputs::service::{rule, term};
use crate::domain::lowering::tables::selections::types::{FlagWord, SelectionAsked};
use crate::domain::lowering::tables::sets::service::{bucket_branches, bucket_column, literals};
use crate::domain::lowering::tables::sets::types::{RelationColumn, SetTests};
use crate::domain::lowering::tables::shapes::service::{operand_kind, test_of};
use crate::domain::lowering::tables::types::RuleFlattening;
use crate::domain::lowering::tables::views::service::relation_read;
use crate::domain::lowering::tables::views::types::RelationView;
use crate::domain::steps::types::{AnchorKinds, ConstantText, TestOperation};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::constants::{LOWERED, RAISED};
use ryo_relations::domain::relations::service::{columns, kind};
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::domain::plan_parts::types::{
    Input, LiteralText, Operand, PlanComparison, QueryLiteral, Side, Test,
};

/// The text a comparison holds a flag column to, when it is an equality or an inequality of a flag column and a text the
/// query wrote. A flag's cell is its stored bits (one raised, nought lowered: `ryo_relations`' `to_bits`), never a word of
/// a text; today's engine takes a flag to equal the word it is shown as (the operators crate's `equality`), and the query
/// language lets a query write no other text against a flag. None for every other comparison.
fn flag_word(
    state: &RuleFlattening<'_>,
    shape: &StepShape,
    comparison: &PlanComparison,
) -> Option<FlagWord> {
    let rule = &rule(state)?.rule;
    let flag = |operand| operand_kind(rule, shape, operand) == Some(CellKind::Flag);
    let (side, written) = match (&comparison.left, &comparison.right) {
        (column, Operand::Literal(QueryLiteral::Text(written))) if flag(column) => {
            (Side::Right, written)
        }
        (Operand::Literal(QueryLiteral::Text(written)), column) if flag(column) => {
            (Side::Left, written)
        }
        (
            Operand::Column(_) | Operand::Parameter(_) | Operand::Literal(_) | Operand::Function(_),
            Operand::Column(_) | Operand::Parameter(_) | Operand::Literal(_) | Operand::Function(_),
        ) => return None,
    };
    let cell = match &*written.0 {
        RAISED => Some(Integer(1)),
        LOWERED => Some(Integer(0)),
        _ => None,
    };
    matches!(comparison.test, Test::Equal | Test::NotEqual).then_some(FlagWord { side, cell })
}

/// A branch with one comparison added as a pin or a test. Outer none: the comparison cannot be written; inner none: the
/// branch holds of no row. A flag column held to the word a flag is shown as is held to the flag's own cell; held to any
/// other text it equals nothing.
fn tested(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    mut branch: FlatBranch,
    comparison: &PlanComparison,
    shape: &StepShape,
) -> Option<Option<FlatBranch>> {
    let test = match (test_of(comparison.test)?, &comparison.right) {
        (TestOperation::Matches(_), Operand::Literal(QueryLiteral::Text(source))) => {
            TestOperation::Matches(regex(drafts, source))
        }
        (test, _) => test,
    };
    let (left, right) = match flag_word(state, shape, comparison) {
        Some(FlagWord {
            side: Side::Right,
            cell: Some(cell),
        }) => {
            let left = term(state, drafts, &mut branch, &comparison.left)?;
            (left, FlatTerm::Number(cell))
        }
        Some(FlagWord {
            side: Side::Left,
            cell: Some(cell),
        }) => {
            let right = term(state, drafts, &mut branch, &comparison.right)?;
            (FlatTerm::Number(cell), right)
        }
        // No flag is shown as the text: an equality holds of no row, an inequality of every row.
        Some(FlagWord { cell: None, .. }) => {
            return Some(Some(branch).filter(|_| comparison.test == Test::NotEqual));
        }
        None => {
            let left = term(state, drafts, &mut branch, &comparison.left)?;
            (left, term(state, drafts, &mut branch, &comparison.right)?)
        }
    };
    Some(compared(branch, test, left, right))
}

/// A branch with every comparison added, each over the branch's own row: the branches it then is. A comparison that holds
/// a node's kind or field to a set of literals gives one branch for each literal when `sets` asks buckets; every other
/// comparison keeps the branch or drops it. None when a comparison cannot be written; no branch: none holds of a row.
pub(crate) fn selected(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    branch: FlatBranch,
    asked: &SelectionAsked<'_>,
) -> Option<Vec<FlatBranch>> {
    let mut held = vec![branch];
    for comparison in asked.comparisons {
        let mut next = Vec::with_capacity(held.len());
        for branch in held {
            let buckets = match asked.sets {
                SetTests::Buckets => {
                    let rule = &rule(state)?.rule;
                    bucket_branches(rule, state.kinds, drafts, &branch, comparison)
                }
                SetTests::Tests => None,
            };
            match buckets {
                Some(pinned) => next.extend(pinned),
                None => next.extend(tested(state, drafts, branch, comparison, &asked.shape)?),
            }
        }
        held = next;
    }
    Some(held)
}

/// Every branch with the comparisons added; the branches that hold of no row are dropped.
pub(crate) fn all_selected(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    branches: Vec<FlatBranch>,
    asked: &SelectionAsked<'_>,
) -> Option<Vec<FlatBranch>> {
    let mut kept = Vec::with_capacity(branches.len());
    for branch in branches {
        kept.extend(selected(state, drafts, branch, asked)?);
    }
    Some(kept)
}

/// The formula of a relation read with a written literal in a column; none for a literal the pack's texts do not hold. In
/// a flag column the literal is the flag's own cell (`flag`), never the text's word.
fn literal_formula(
    state: &RuleFlattening<'_>,
    drafts: &Drafts,
    read: RelationColumn,
    written: &LiteralText,
) -> Option<Emptiness> {
    if kind(read.column) == CellKind::Flag {
        let cell = match &*written.0 {
            RAISED => CellWord(1),
            LOWERED => CellWord(0),
            _ => return None,
        };
        return Some(literal_read(read.relation, read.column, cell, None));
    }
    let number = drafts.text_numbers.get(written)?;
    let kinds = kind_bits(state.kinds, &ConstantText(written.0.clone()))
        .map(|bits| AnchorKinds([bits].into()));
    let value = text_word(*number);
    Some(literal_read(read.relation, read.column, value, kinds))
}

/// The emptiness of a selection. Of one read of a relation (`read`: what the selection's input was before the comparisons,
/// a relation itself or a step that is a view of one) it is that of the relation read with the first literal a comparison
/// holds a column to: a pin's formula, or for a node's kind or field held to a set of literals the `Any` of each
/// literal's. Else its input's: of an input that is no such read, and of a read whose formula already names a value.
fn selection_emptiness(
    state: &RuleFlattening<'_>,
    drafts: &Drafts,
    read: Option<&RelationView>,
    comparisons: &[PlanComparison],
    from: Emptiness,
) -> Emptiness {
    let Some(read) = read.filter(|_| matches!(from, Emptiness::Unknown | Emptiness::Row(_, None)))
    else {
        return from;
    };
    let (relation, listed) = (read.relation, columns(read.relation));
    for comparison in comparisons {
        let Operand::Column(column) = &comparison.left else {
            continue;
        };
        // The column of the relation the selection's column is: the place of its cell in the relation's row.
        let cell = read.row.get(usize::from(*column));
        let place = cell.and_then(|cell| match cell {
            FlatTerm::Variable(_) => read.cells.iter().position(|held| held == cell),
            FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => None,
        });
        let Some(column) = place.and_then(|place| listed.get(place).copied()) else {
            continue;
        };
        let read = RelationColumn { relation, column };
        if let (Some(TestOperation::Equal), Operand::Literal(QueryLiteral::Text(written))) =
            (test_of(comparison.test), &comparison.right)
            && let Some(formula) = literal_formula(state, drafts, read, written)
        {
            return formula;
        }
        // A kind or a field held to a set of literals: a row of some literal's bucket.
        let set = rule(state)
            .filter(|_| bucket_column(relation, column).is_some())
            .and_then(|rule| literals(&rule.rule, comparison));
        let parts: Option<Vec<Emptiness>> = set.and_then(|set| {
            let each = set.iter();
            each.map(|written| literal_formula(state, drafts, read, written))
                .collect()
        });
        if let Some(parts) = parts {
            return Emptiness::Any(parts.into());
        }
    }
    from
}

/// The comparisons of a selection over the rows of `read`, with the shape of those rows.
pub(crate) fn asked_of<'a>(
    state: &RuleFlattening<'_>,
    read: Input,
    comparisons: &'a [PlanComparison],
    sets: SetTests,
) -> Option<SelectionAsked<'a>> {
    let shape = input_shape(&rule(state)?.shapes, read)?;
    Some(SelectionAsked {
        comparisons,
        shape,
        sets,
    })
}

/// A read with a selection's comparisons added: its branches selected, and its emptiness the selection's.
pub(crate) fn selection(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    mut read: Flattened,
    asked: &SelectionAsked<'_>,
) -> Option<Flattened> {
    let before = relation_read(&read.branches);
    read.branches = all_selected(state, drafts, read.branches, asked)?;
    let from = std::mem::take(&mut read.emptiness);
    read.emptiness = selection_emptiness(state, drafts, before.as_ref(), asked.comparisons, from);
    Some(read)
}
