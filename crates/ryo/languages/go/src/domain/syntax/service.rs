//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no ports. It maps a kind to the number the
//! element table holds and back, says its name, and moves about a file's syntax tree: a position is an element of the table; its kind
//! is the language's own.

use ryo::languages::{self};

use super::types::{Kind, Numbered, Output};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Token(Token::Illegal));

impl Output for Numbered<'_, '_> {
    fn enter(&mut self, kind: Kind, start: languages::Offset) {
        languages::node_table::enter_node(self.builder, kind_bits(kind), languages::FieldCode(0), start);
    }

    /// A token's text in the numbered table is the text written in the file, so what the library decoded of a string is not kept
    /// there: the value of a literal is read from its written text where a relation needs it.
    fn token(&mut self, kind: Kind, start: languages::Offset, end: languages::Offset, _decoded: Option<&str>) {
        languages::node_table::token(self.builder, kind_bits(kind), languages::FieldCode(0), start, end);
    }

    fn leave(&mut self, end: languages::Offset) {
        languages::node_table::leave_node(self.builder, end);
    }

    fn diagnose(&mut self, message: &str, start: languages::Offset, end: languages::Offset) {
        languages::node_table::diagnose(self.builder, start, end, languages::NameText(message));
    }
}
