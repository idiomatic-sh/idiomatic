//! The services of the `selections` part: a branch with every comparison of a selection added, each over the branch's own
//! row, and what a file must hold for the selection to have a row. A comparison that holds a node's kind or field to a set
//! of literals gives one branch for each literal (`sets`); every other comparison is a pin or a test of the branch.

use crate::domain::lowering::observing::service::{exactly, input_kinds};
use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::definitions::service::{kind_bits, regex, text_word};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::literal_read;
use crate::domain::lowering::flattening::service::compared;
use crate::domain::lowering::flattening::types::{FlatBranch, FlatTerm};
use crate::domain::lowering::tables::inputs::service::{rule, term};
use crate::domain::lowering::tables::literals::service::{held_literal, term_cell};
use crate::domain::lowering::tables::literals::types::LiteralCell;
use crate::domain::lowering::tables::selections::types::SelectionAsked;
use crate::domain::lowering::tables::sets::service::{bucket_branches, bucket_column, literals};
use crate::domain::lowering::tables::sets::types::{RelationColumn, SetTests};
use crate::domain::lowering::tables::shapes::service::{operand_kind, test_of};
use crate::domain::lowering::tables::types::RuleFlattening;
use crate::domain::lowering::tables::views::service::relation_read;
use crate::domain::lowering::tables::views::types::RelationView;
use crate::domain::steps::types::{AnchorKinds, ConstantText, SideKind, TestOperation, TestSides};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::constants::{LOWERED, RAISED};
use ryo_relations::domain::relations::service::{columns, kind, vocabulary_of};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::Vocabulary;
use ryo_values::domain::plan_parts::types::{
    Input, LiteralText, Operand, PlanComparison, QueryLiteral, Side, Test,
};

/// What each side of a numeric comparison holds: a whole number, or a text the test reads as the number it writes. None
/// for an operand the lowering writes no operand for.
fn sides(
    state: &RuleFlattening<'_>,
    asked: &SelectionAsked<'_>,
    comparison: &PlanComparison,
) -> Option<TestSides> {
    let selected = &rule(state)?.rule;
    let side = |operand: &Operand| match operand_kind(selected, &asked.shape, operand)? {
        CellKind::Number => Some(SideKind::Number),
        CellKind::Symbol | CellKind::Node | CellKind::File | CellKind::Span | CellKind::Flag => Some(SideKind::Text),
    };
    Some(TestSides {
        left: side(&comparison.left)?,
        right: side(&comparison.right)?,
    })
}

/// A branch with one comparison added as a pin or a test: the branches it then is. None: the comparison cannot be
/// written; no branch: it holds of no row. A text a column is held to is that column's cell (`literals`): a flag's own
/// cell, a closed set's word; a text no cell of the column is shown as equals nothing.
fn tested(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    mut branch: FlatBranch,
    comparison: &PlanComparison,
    asked: &SelectionAsked<'_>,
) -> Option<Vec<FlatBranch>> {
    let test = match (test_of(comparison.test)?, &comparison.right) {
        (TestOperation::Matches(_), Operand::Literal(QueryLiteral::Text(source))) => {
            TestOperation::Matches(regex(drafts, source))
        }
        // A numeric test reads each side as a number or as a text: the kinds are written here, where the operands are
        // still columns and literals (SPIKE ryo-fmx3.1, Q8).
        (TestOperation::NumericAtLeast(_), _) => TestOperation::NumericAtLeast(sides(state, asked, comparison)?),
        (TestOperation::Successor(_), _) => TestOperation::Successor(sides(state, asked, comparison)?),
        (test, _) => test,
    };
    // A file's cell is its number in the run, never the word of a text: a file held equal to a text is its path held to it,
    // which is a test of the path's text (an expression of the whole text, as `matches` reads the path).
    if comparison.test == Test::Equal
        && let Some((column, written)) = file_held(state, asked, comparison)
    {
        let whole = exactly(&written.0);
        let left = term(state, drafts, &mut branch, column)?;
        let right = term(state, drafts, &mut branch, &Operand::Literal(QueryLiteral::Text(whole.clone())))?;
        let test = TestOperation::Matches(regex(drafts, &whole));
        return Some(compared(branch, test, left, right).into_iter().collect());
    }
    let Some(held) = held_literal(state, drafts, asked.read, &asked.shape, comparison) else {
        let left = term(state, drafts, &mut branch, &comparison.left)?;
        let right = term(state, drafts, &mut branch, &comparison.right)?;
        return Some(compared(branch, test, left, right).into_iter().collect());
    };
    let column = match held.side {
        Side::Right => term(state, drafts, &mut branch, &comparison.left)?,
        Side::Left => term(state, drafts, &mut branch, &comparison.right)?,
    };
    let with = |branch: FlatBranch, cell: FlatTerm| match held.side {
        Side::Right => compared(branch, test, column, cell),
        Side::Left => compared(branch, test, cell, column),
    };
    Some(match (held.cell, comparison.test) {
        (LiteralCell::Exactly(cell), _) => with(branch, cell).into_iter().collect(),
        // No cell of the column is shown as the text: an equality holds of no row, an inequality of every row.
        (LiteralCell::Never, Test::NotEqual) => vec![branch],
        (LiteralCell::Never, _) => Vec::new(),
        // The column holds some one of the cells: unequal to each, or equal to one, a branch each.
        (LiteralCell::Either(cells), Test::NotEqual) => {
            let mut kept = Some(branch);
            for cell in cells {
                kept = kept.and_then(|branch| with(branch, cell));
            }
            kept.into_iter().collect()
        }
        (LiteralCell::Either(cells), _) => cells
            .into_iter()
            .filter_map(|cell| with(branch.clone(), cell))
            .collect(),
    })
}

/// The file column of a comparison that holds it to a text, and the text: none for any other comparison.
fn file_held<'c>(
    state: &RuleFlattening<'_>,
    asked: &SelectionAsked<'_>,
    comparison: &'c PlanComparison,
) -> Option<(&'c Operand, &'c LiteralText)> {
    let (column, written) = match (&comparison.left, &comparison.right) {
        (column @ Operand::Column(_), Operand::Literal(QueryLiteral::Text(written)))
        | (Operand::Literal(QueryLiteral::Text(written)), column @ Operand::Column(_)) => (column, written),
        _ => return None,
    };
    (operand_kind(&rule(state)?.rule, &asked.shape, column)? == CellKind::File).then_some((column, written))
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
                None => next.extend(tested(state, drafts, branch, comparison, asked)?),
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
    // A file's cell is no word of a text: the file's rows say nothing of it.
    if kind(read.column) == CellKind::File {
        return None;
    }
    if kind(read.column) == CellKind::Flag {
        let cell = match &*written.0 {
            RAISED => CellWord(1),
            LOWERED => CellWord(0),
            _ => return None,
        };
        return Some(literal_read(read.relation, read.column, cell, None));
    }
    // A closed set's column holds the word's own symbol, never a word of the file.
    if let Vocabulary::Closed(block) = vocabulary_of(read.column) {
        let FlatTerm::Number(symbol) = term_cell(block, &written.0)? else {
            return None;
        };
        let cell = CellWord(symbol.0.cast_unsigned());
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
    let held = rule(state)?;
    let shape = input_kinds(&held.rule, &held.shapes, read)?;
    Some(SelectionAsked {
        comparisons,
        read,
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
