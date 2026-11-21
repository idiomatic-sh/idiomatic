//! The services of the `literals` part: a text literal compared with a column, coded as the cell that column's kind
//! holds. A filled relation's cell is "a word's code for a text cell, the stored bits of any other cell": so a text is a
//! word only where the column holds words a file wrote. A flag column holds one or nought (the text `true` or `false`); a
//! column of a closed set of words (`vocabulary_of`: a declaration's kind, a parameter's) holds the word's own symbol,
//! the same in every file, which is no word of any file. Today's engine compares what each cell is SHOWN as; here the
//! literal becomes the cell, and a text no cell of the column is shown as equals nothing: never a silent miss.

use super::types::{HeldLiteral, LiteralCell};
use crate::domain::lowering::definitions::service::text;
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::types::FlatTerm;
use crate::domain::lowering::nodes::service::input_shape;
use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::tables::inputs::service::rule;
use crate::domain::lowering::tables::shapes::service::operand_kind;
use crate::domain::lowering::tables::types::RuleFlattening;
use ryo_relations::domain::declaration_kind::types::DeclarationKind;
use ryo_relations::domain::dependency_kind::types::DependencyKind;
use ryo_relations::domain::dependency_origin::types::DependencyOrigin;
use ryo_relations::domain::mapping_kind::types::MappingKind;
use ryo_relations::domain::parameter_kind::types::ParameterKind;
use ryo_relations::domain::relations::constants::{LOWERED, RAISED};
use ryo_relations::domain::relations::service::{columns, to_bits, vocabulary_of};
use ryo_relations::domain::relations::types::{CellKind, Column, Integer, Value};
use ryo_relations::domain::terms::service::{term_symbol, words_of};
use ryo_relations::domain::terms::types::{Vocabulary, WordBlock};
use ryo_relations::domain::value_kind::types::ValueKind;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, LiteralText, Operand, PlanComparison, QueryLiteral, Side, Test,
};
use ryo_values::domain::plans::types::Operation;

/// The closed sets of words a column may hold.
const BLOCKS: [WordBlock; 6] = [
    WordBlock::Declaration,
    WordBlock::Parameter,
    WordBlock::Value,
    WordBlock::Mapping,
    WordBlock::Dependency,
    WordBlock::Origin,
];

/// The cell a word of a closed set is: its symbol's stored bits, the same in every file. None for a text that is no word
/// of the set.
#[must_use]
pub(crate) fn term_cell(block: WordBlock, written: &str) -> Option<FlatTerm> {
    let place = words_of(block).iter().position(|word| word.0 == written)?;
    let place = u8::try_from(place).ok()?;
    let symbol = match block {
        WordBlock::Declaration => DeclarationKind::from_repr(place).map(term_symbol),
        WordBlock::Parameter => ParameterKind::from_repr(place).map(term_symbol),
        WordBlock::Value => ValueKind::from_repr(place).map(term_symbol),
        WordBlock::Mapping => MappingKind::from_repr(place).map(term_symbol),
        WordBlock::Dependency => DependencyKind::from_repr(place).map(term_symbol),
        WordBlock::Origin => DependencyOrigin::from_repr(place).map(term_symbol),
        WordBlock::Unknown => None,
    }?;
    let bits = to_bits(Value::Symbol(symbol));
    Some(FlatTerm::Number(Integer(bits.0.cast_signed())))
}

/// The column of a relation a column of an input's rows holds unchanged, traced through the steps that keep a column as it
/// is. None when the plan does not say: a column a step made, or one of a step not traced.
fn source_column(state: &RuleFlattening<'_>, input: Input, column: ColumnIndex) -> Option<Column> {
    let rule = rule(state)?;
    let step = match input {
        Input::Relation(relation) => return columns(relation).get(usize::from(column)).copied(),
        Input::Step(step) => step,
        Input::BoundObservation(_) => return None,
    };
    let width = |input: &Input| input_shape(&rule.shapes, *input).map(|shape| shape.0.len());
    let within = |from: &Input| (usize::from(column) < width(from)?).then_some(column);
    match &rule.rule.plan.steps.get(usize::from(step))?.operation {
        Operation::Select { from, kept, .. } if kept.is_empty() => {
            source_column(state, *from, column)
        }
        Operation::Select { from, kept, .. } => {
            source_column(state, *from, *kept.get(usize::from(column))?)
        }
        Operation::Project { from, columns, .. } => {
            source_column(state, *from, *columns.get(usize::from(column))?)
        }
        Operation::Join {
            left,
            right,
            output,
            view,
            ..
        } => {
            let from = output.get(usize::from(column))?;
            match from.side {
                Side::Left => source_column(state, *left, from.column),
                Side::Right => source_column(state, *right, view_source(view, from.column)?),
            }
        }
        // Both sides of a union must say the same column: else what the column holds is not known.
        Operation::Union { left, right } => {
            let found = source_column(state, *left, column)?;
            (source_column(state, *right, column)? == found).then_some(found)
        }
        Operation::Antijoin { left: from, .. }
        | Operation::GroupBound { from, .. }
        | Operation::Difference { left: from, .. } => source_column(state, *from, column),
        Operation::Extend { from, .. }
        | Operation::GroupOrdinal { from, .. }
        | Operation::Axis { from, .. } => source_column(state, *from, within(from)?),
        Operation::GroupCount { from, keys, .. } => {
            source_column(state, *from, *keys.get(usize::from(column))?)
        }
        Operation::Intersection { .. }
        | Operation::AsOf { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Closure { .. }
        | Operation::Follow { .. }
        | Operation::Cycles { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::Words { .. }
        | Operation::SpanBetween { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => None,
    }
}

/// Whether a text a written row holds is compared with a column of `input`'s rows as the text's own word: the column is
/// one of words a file wrote. A closed set's column holds the word's symbol and a column the plan does not trace may:
/// the text would be compared with another cell, so such a join is not lowered.
pub(crate) fn text_compared(state: &RuleFlattening<'_>, input: Input, column: ColumnIndex) -> bool {
    source_column(state, input, column).is_some_and(|traced| {
        vocabulary_of(traced) == Vocabulary::Open && kind(traced) == CellKind::Symbol
    })
}

/// What a text is as a cell of a symbol column read from `input`: of a closed set's column, the word's symbol; of a
/// column of words a file wrote, the text's own word (none: as written); of a column the plan does not trace, either,
/// when the text is a word of some closed set.
fn symbol_cell(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    input: Input,
    column: &Operand,
    written: &LiteralText,
) -> Option<LiteralCell> {
    let traced = match column {
        Operand::Column(column) => source_column(state, input, *column),
        Operand::Literal(_) | Operand::Parameter(_) | Operand::Function(_) => return None,
    };
    match traced.map(vocabulary_of) {
        Some(Vocabulary::Closed(block)) => {
            Some(term_cell(block, &written.0).map_or(LiteralCell::Never, LiteralCell::Exactly))
        }
        Some(Vocabulary::Open) => None,
        None => {
            let words = BLOCKS
                .iter()
                .filter_map(|block| term_cell(*block, &written.0));
            let mut cells = vec![FlatTerm::Text(text(drafts, state.kinds, written))];
            cells.extend(words);
            (cells.len() > 1).then_some(LiteralCell::Either(cells))
        }
    }
}

/// The text an equality or an inequality holds a column of `input`'s rows to, as that column's cell, when the cell is not
/// the text's own word: a flag column's (one for `true`, nought for `false`), a closed set's (the word's symbol). None for
/// every other comparison: its operands are written as they are.
pub(crate) fn held_literal(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    input: Input,
    shape: &StepShape,
    comparison: &PlanComparison,
) -> Option<HeldLiteral> {
    if !matches!(comparison.test, Test::Equal | Test::NotEqual) {
        return None;
    }
    let (side, column, written) = match (&comparison.left, &comparison.right) {
        (column @ Operand::Column(_), Operand::Literal(QueryLiteral::Text(written))) => {
            (Side::Right, column, written)
        }
        (Operand::Literal(QueryLiteral::Text(written)), column @ Operand::Column(_)) => {
            (Side::Left, column, written)
        }
        (
            Operand::Column(_) | Operand::Parameter(_) | Operand::Literal(_) | Operand::Function(_),
            Operand::Column(_) | Operand::Parameter(_) | Operand::Literal(_) | Operand::Function(_),
        ) => return None,
    };
    let cell = match operand_kind(&rule(state)?.rule, shape, column)? {
        CellKind::Flag => match &*written.0 {
            RAISED => LiteralCell::Exactly(FlatTerm::Number(Integer(1))),
            LOWERED => LiteralCell::Exactly(FlatTerm::Number(Integer(0))),
            _ => LiteralCell::Never,
        },
        CellKind::Symbol => symbol_cell(state, drafts, input, column, written)?,
        CellKind::Node | CellKind::File | CellKind::Span | CellKind::Number => return None,
    };
    Some(HeldLiteral { side, cell })
}
