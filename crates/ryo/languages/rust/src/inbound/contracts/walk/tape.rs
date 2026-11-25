//! The walk's writing of the parser's steps: each is kept on the tape and handed to the file's elements as it comes, and when the
//! parse is over the tape is written into the node table, each element in the field its role among its siblings gives it.

use crate::outbound::syntax::interface::Output;
use crate::outbound::syntax::types::{Tape, Written};
use ryo::languages::{self as l, node_table as t};
use std::fmt::Write as _;

/// Writes each step into the file's elements and onto the tape.
pub(super) struct Tee<'a, 'b> {
    /// The steps and their texts.
    pub(super) tape: &'a mut Tape,
    /// The file's elements.
    pub(super) elements: &'a mut l::TableBuilder<'b>,
}

impl Tee<'_, '_> {
    fn kept(&mut self, text: &str) -> (u32, u32) {
        let from = u32::try_from(self.tape.texts.len()).unwrap_or(u32::MAX);
        self.tape.texts.push_str(text);
        (from, u32::try_from(self.tape.texts.len()).unwrap_or(u32::MAX))
    }
}

impl Output for Tee<'_, '_> {
    fn enter(&mut self, kind: l::KindBits, start: l::Offset) {
        self.tape.steps.push(Written::Enter(kind, start));
        self.elements.enter(kind, start);
    }
    fn leave(&mut self, end: l::Offset) {
        self.tape.steps.push(Written::Leave(end));
        self.elements.leave(end);
    }
    fn token(&mut self, kind: l::KindBits, start: l::Offset, end: l::Offset, decoded: Option<l::NameText<'_>>) {
        let text = decoded.map(|text| self.kept(text.0));
        self.tape.steps.push(Written::Token(kind, start, end, text));
        self.elements.token(kind, start, end, decoded);
    }
    fn property(&mut self, key: l::PropertyId, value: l::NameText<'_>) {
        let text = self.kept(value.0);
        self.tape.steps.push(Written::Property(key, text));
        self.elements.property(key, value);
    }
    fn formatted(&mut self, key: l::PropertyId, value: std::fmt::Arguments<'_>) -> std::fmt::Result {
        let from = u32::try_from(self.tape.texts.len()).unwrap_or(u32::MAX);
        self.tape.texts.write_fmt(value)?;
        let to = u32::try_from(self.tape.texts.len()).unwrap_or(u32::MAX);
        self.tape.steps.push(Written::Property(key, (from, to)));
        self.elements.formatted(key, value)
    }
    fn diagnostic(&mut self, message: &str, start: l::Offset, end: l::Offset) {
        let text = self.kept(message);
        self.tape.steps.push(Written::Diagnostic(text, start, end));
        self.elements.diagnostic(message, start, end);
    }
    fn adopt(&mut self, host: l::Position) {
        self.tape.steps.push(Written::Adopt(host));
        self.elements.adopt(host);
    }
    fn origin(&mut self, at: l::Position, host: l::Position, expansion: l::ExpansionId) {
        self.elements.origin(at, host, expansion);
    }
    fn release(&mut self) {
        self.tape.steps.push(Written::Release);
        self.elements.release();
    }
}

/// Writes the tape into the node table the builder builds, the element at each place in the field `field` gives it. The first
/// element of an expansion's tree embeds it under the call it expands, as the native reader does.
pub(super) fn replay(tape: &Tape, builder: &mut l::NodeTableBuilder<'_>, field: impl Fn(usize) -> u16) {
    let text = |(from, to): (u32, u32)| l::NameText(&tape.texts[from as usize..to as usize]);
    let mut host: Option<l::Position> = None;
    let mut depth = 0_usize;
    let mut place = 0_usize;
    let mut open = |builder: &mut l::NodeTableBuilder<'_>, depth: usize, host: Option<l::Position>, kind, start| {
        if depth == 0
            && let Some(host) = host
        {
            let _ = t::embed(
                builder,
                host,
                l::EmbeddedText { text: l::NameText(""), mapping: l::EmbeddedMapping::Exact(l::Offset(0)) },
            );
        }
        t::enter_node(builder, kind, l::FieldCode(field(place)), start);
        place += 1;
    };
    for step in &tape.steps {
        match *step {
            Written::Enter(kind, start) => {
                open(builder, depth, host, kind, start);
                depth += 1;
            }
            Written::Leave(end) => {
                depth -= 1;
                t::leave_node(builder, end);
            }
            Written::Token(kind, start, end, decoded) => {
                open(builder, depth, host, kind, start);
                if let Some(decoded) = decoded {
                    t::node_property(builder, l::PropertyId(0), text(decoded));
                }
                t::leave_node(builder, end);
            }
            Written::Property(key, value) => t::node_property(builder, key, text(value)),
            Written::Diagnostic(message, start, end) => t::diagnose(builder, start, end, text(message)),
            Written::Adopt(call) => host = Some(call),
            Written::Release => host = None,
        }
    }
}
