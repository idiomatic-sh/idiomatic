//! The services of the `calls` part: an operation that is one per-row meaning of the operators crate. Each row of its
//! input goes on with what the meaning gives. The arguments are the operation's column operands in the order its fields
//! are declared, then its constant fields as constants; a regular expression is the meaning's own number.

use crate::domain::lowering::definitions::service::{fresh_variable, regex, text};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::flattening::types::{FlatAtom, FlatTerm};
use crate::domain::lowering::tables::inputs::service::input;
use crate::domain::lowering::tables::types::{InputWant, OperationCall, RuleFlattening};
use crate::domain::steps::types::CallMeaning;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::Input;
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
        | Operation::Capture { .. }
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
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. } => None,
    }
}
