//! The node table the walk writes: the tree of the writer's callbacks and tokens, numbered as the engine's table numbers it. The role
//! of a part of a declaration (its name, its value, its `!important`) is known only when the declaration ends, so what a declaration
//! holds is kept until it closes and written then, each part in its role.

use super::writer::interface::Out;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::file_view::service::record;
use crate::domain::syntax::service::{declaration_fields, field_bits, kind_bits};
use crate::domain::syntax::types::{Field, Kind, Word};
use crate::domain::tokens::types::Token;
use ryo::languages::{FieldCode, NameText, NodeTableBuilder, Offset, node_table};

/// One thing the writer wrote inside a declaration.
#[derive(Debug)]
enum Op {
    /// A node opened.
    Enter(Kind, Offset),
    /// A node closed.
    Leave(Offset),
    /// A token, with the text the library decoded from it.
    Token(Kind, Offset, Offset, Option<String>),
}

/// What a declaration holds, kept until it closes: what was written inside it, and how many nodes inside it are open.
#[derive(Debug, Default)]
struct Held {
    /// What was written, in order.
    ops: Vec<Op>,
    /// How many nodes opened inside the declaration are open.
    depth: usize,
}

/// The tree written into the builder of a file's node table.
pub(super) struct Numbered<'b, 'w, 't, 'd> {
    /// What the tree is written through.
    pub builder: &'b mut NodeTableBuilder<'w>,
    /// The bytes of the file, which a token's text is read from.
    text: &'t [u8],
    /// How many things have been said of the file.
    pub said: usize,
    /// The declaration open, with what it holds so far.
    held: Option<Held>,
    /// What the library decoded of the tokens, kept beside the table.
    decodings: &'d mut crate::domain::file_view::types::Decodings,
}

impl<'b, 'w, 't, 'd> Numbered<'b, 'w, 't, 'd> {
    /// A tree written through `builder`, whose stylesheet node is open, over the bytes `text` of the file.
    pub(super) fn new(builder: &'b mut NodeTableBuilder<'w>, text: &'t [u8], decodings: &'d mut crate::domain::file_view::types::Decodings) -> Self {
        Self {
            decodings,
            builder,
            text,
            said: 0,
            held: None,
        }
    }

    /// The text of a span of the file.
    fn spelled(&self, from: Offset, to: Offset) -> &'t str {
        let (from, to) = (
            usize::try_from(from.0).unwrap_or(usize::MAX),
            usize::try_from(to.0).unwrap_or(usize::MAX),
        );
        self.text
            .get(from..to)
            .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
            .unwrap_or_default()
    }

    /// Writes what a closed declaration held, each of its parts in its role, and closes it at `at`.
    fn flush(&mut self, held: Held, at: Offset) {
        let mut depth = 0_usize;
        // The parts of the declaration itself: the nodes and tokens directly in it, by where they are among its ops.
        let mut parts: Vec<(usize, Word<'_>)> = Vec::new();
        for (place, op) in held.ops.iter().enumerate() {
            match op {
                Op::Enter(kind, _) => {
                    if depth == 0 {
                        parts.push((place, Word { kind: *kind, spelled: "", decoded: None }));
                    }
                    depth += 1;
                }
                Op::Leave(_) => depth = depth.saturating_sub(1),
                Op::Token(kind, from, to, decoded) => {
                    if depth == 0 {
                        parts.push((
                            place,
                            Word {
                                kind: *kind,
                                spelled: self.spelled(*from, *to),
                                decoded: decoded.as_deref(),
                            },
                        ));
                    }
                }
            }
        }
        let words: Vec<Word<'_>> = parts
            .iter()
            .map(|(_, word)| *word)
            .filter(|word| {
                !matches!(
                    word.kind,
                    Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment)
                )
            })
            .collect();
        let fields = declaration_fields(&words);
        // The role of each part that is a word, by its place among the ops.
        let mut roles: std::collections::HashMap<usize, Field> = std::collections::HashMap::new();
        let mut next = 0_usize;
        for (place, word) in &parts {
            if matches!(
                word.kind,
                Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment)
            ) {
                continue;
            }
            if let Some(field) = fields.get(next) {
                let _ = roles.insert(*place, *field);
            }
            next += 1;
        }
        for (place, op) in held.ops.iter().enumerate() {
            let field = FieldCode(field_bits(roles.get(&place).copied().unwrap_or(Field::None)).0);
            match op {
                Op::Enter(kind, start) => {
                    node_table::enter_node(self.builder, kind_bits(*kind), field, *start);
                }
                Op::Leave(end) => node_table::leave_node(self.builder, *end),
                Op::Token(kind, from, to, None) => {
                    node_table::token(self.builder, kind_bits(*kind), field, *from, *to);
                }
                Op::Token(kind, from, to, Some(text)) => {
                    node_table::token(self.builder, kind_bits(*kind), field, *from, *to);
                    record(self.decodings, *from, *to, text);
                }
            }
        }
        node_table::leave_node(self.builder, at);
    }
}

impl Out for Numbered<'_, '_, '_, '_> {
    fn enter(&mut self, kind: Kind, at: Offset) {
        if let Some(held) = &mut self.held {
            held.ops.push(Op::Enter(kind, at));
            held.depth += 1;
            return;
        }
        node_table::enter_node(self.builder, kind_bits(kind), FieldCode(field_bits(Field::None).0), at);
        if kind == Kind::Node(Node::Declaration) {
            self.held = Some(Held::default());
        }
    }

    fn leave(&mut self, at: Offset) {
        match self.held.take() {
            Some(mut held) if held.depth > 0 => {
                held.ops.push(Op::Leave(at));
                held.depth -= 1;
                self.held = Some(held);
            }
            Some(held) => self.flush(held, at),
            None => node_table::leave_node(self.builder, at),
        }
    }

    fn token(&mut self, kind: Kind, from: Offset, to: Offset, decoded: Option<&str>) {
        if let Some(held) = &mut self.held {
            held.ops.push(Op::Token(kind, from, to, decoded.map(str::to_owned)));
            return;
        }
        let field = FieldCode(field_bits(Field::None).0);
        match decoded {
            Some(text) => {
                node_table::token(self.builder, kind_bits(kind), field, from, to);
                record(self.decodings, from, to, text);
            }
            None => node_table::token(self.builder, kind_bits(kind), field, from, to),
        }
    }

    fn diagnose(&mut self, message: &str, from: Offset, to: Offset) {
        node_table::diagnose(self.builder, from, to, NameText(message));
        self.said += 1;
    }
}
