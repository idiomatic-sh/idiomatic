//! The services of the `calls` part: an operation that is one per-row meaning of the operators crate. Each row of its
//! input goes on with what the meaning gives. The arguments are the operation's column operands in the order its fields
//! are declared, then its constant fields as constants; a regular expression is the meaning's own number. A function a
//! comparison applies is a call too: its arguments in their order, one cell given.

use crate::domain::lowering::definitions::service::{fresh_variable, regex, text};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::flattening::types::{FlatAtom, FlatTerm};
use crate::domain::lowering::tables::inputs::service::input;
use crate::domain::lowering::tables::shapes::service::{
    capture_slots, search_kinds, source_search,
};
use crate::domain::lowering::tables::types::{InputWant, OperationCall, RuleFlattening};
use crate::domain::steps::types::CallMeaning;
use ryo_relations::domain::relations::types::{Count, Integer};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plans::types::Operation;

/// Each branch of an input with one call: it reads the columns named, then the constants, and appends what it gives.
fn called(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    from: Input,
    call: OperationCall<'_>,
) -> Option<Flattened> {
    let mut read = input(state, drafts, from, InputWant::Inline)?;
    for branch in &mut read.branches {
        let mut arguments =
            Vec::with_capacity(call.reads.len().saturating_add(call.constants.len()));
        for column in call.reads {
            arguments.push(*branch.row.get(usize::from(*column))?);
        }
        arguments.extend(call.constants.iter().copied());
        let outputs: Vec<FlatTerm> = (0..call.gives.0)
            .map(|_| FlatTerm::Variable(fresh_variable(drafts)))
            .collect();
        branch.row.extend(outputs.iter().copied());
        branch.atoms.push(FlatAtom::Called {
            meaning: call.meaning,
            arguments,
            outputs,
        });
    }
    Some(read)
}

/// The branches of an operation that is one per-row meaning; none for any other operation.
pub(crate) fn call_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    operation: &Operation,
) -> Option<Flattened> {
    match operation {
        Operation::Replace {
            from,
            text: read,
            pattern,
            replacement,
        } => {
            let meaning = CallMeaning::Replace(regex(drafts, pattern));
            let with = FlatTerm::Text(text(drafts, state.kinds, replacement));
            let call = OperationCall {
                meaning,
                reads: &[*read],
                constants: &[with],
                gives: Count(1),
            };
            called(state, drafts, *from, call)
        }
        Operation::SpanBounds { from, span } => {
            let call = OperationCall {
                meaning: CallMeaning::SpanBounds,
                reads: &[*span],
                constants: &[],
                gives: Count(2),
            };
            called(state, drafts, *from, call)
        }
        Operation::SourceMatches {
            from,
            span,
            base_line,
            pattern,
            capture,
            ..
        } => {
            let search = source_search(operation)?;
            let mut reads = vec![*span];
            reads.extend(base_line.iter().copied());
            let named: Vec<FlatTerm> = capture
                .iter()
                .map(|name| FlatTerm::Text(text(drafts, state.kinds, name)))
                .collect();
            let call = OperationCall {
                meaning: CallMeaning::SourceMatches(regex(drafts, pattern), search),
                reads: &reads,
                constants: &named,
                gives: Count(search_kinds(search).len()),
            };
            called(state, drafts, *from, call)
        }
        Operation::Capture {
            from,
            path,
            pattern,
        } => {
            let written = LiteralText(pattern.0.clone());
            let with = FlatTerm::Text(text(drafts, state.kinds, &written));
            let call = OperationCall {
                meaning: CallMeaning::Capture,
                reads: &[*path],
                constants: &[with],
                gives: capture_slots(pattern),
            };
            called(state, drafts, *from, call)
        }
        Operation::SpanBetween { from, start, end } => {
            let call = OperationCall {
                meaning: CallMeaning::SpanBetween,
                reads: &[*start, *end],
                constants: &[],
                gives: Count(1),
            };
            called(state, drafts, *from, call)
        }
        Operation::FitsRendered {
            from,
            texts,
            literals,
            span,
            max_line,
            tab_width,
        } => {
            // The text columns, then the span; then the literals, the widest line and the tab stop as constants.
            let mut reads = texts.clone();
            reads.push(*span);
            let mut constants: Vec<FlatTerm> = literals
                .iter()
                .map(|literal| FlatTerm::Text(text(drafts, state.kinds, literal)))
                .collect();
            constants.push(FlatTerm::Number(Integer(max_line.0)));
            constants.push(FlatTerm::Number(Integer(i64::from(tab_width.0))));
            let call = OperationCall {
                meaning: CallMeaning::FitsRendered,
                reads: &reads,
                constants: &constants,
                gives: Count(0),
            };
            called(state, drafts, *from, call)
        }
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Rank { .. }
        | Operation::OrderedMatch { .. }
        | Operation::AsOf { .. }
        | Operation::GroupBound { .. }
        | Operation::GroupOrdinal { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Follow { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::RenderSyntax { .. } => None,
    }
}
