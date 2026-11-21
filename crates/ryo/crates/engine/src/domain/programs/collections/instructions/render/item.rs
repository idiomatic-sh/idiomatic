//! `Render`: the language's source form of the node in a register, in the word's syntax role, written to its output; no
//! row when the node has none in the role. The rendering is the fill side's (`stages::calling::service::rendered`): a
//! `Render` word and a `Render` stage have one definition.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::deciding::service::render_of;
use crate::domain::code::small_codes::service::context_of;
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::executing::sources::service::faulted;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, set};
use crate::domain::stages::calling::service::rendered;

/// The `Render` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Render;

impl InstructionsItem for Render {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let asked = render_of(word);
        let Some(context) = context_of(asked.context) else {
            faulted(
                frame.state,
                FilledError::Code {
                    named: MissingCode::Word,
                },
            );
            return Flow::Back;
        };
        let node = cell(frame, asked.node);
        let fill = &mut *frame.fill;
        match rendered(&mut *fill.facts, &mut *fill.words, node, context) {
            Ok(Some(text)) => {
                set(frame, asked.out, text);
                Flow::Next
            }
            Ok(None) => Flow::Back,
            Err(fault) => {
                faulted(frame.state, fault);
                Flow::Back
            }
        }
    }
}
