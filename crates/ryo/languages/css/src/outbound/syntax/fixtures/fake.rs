//! The fake of the `syntax` port: what the language's tests stand in for it with.

use super::super::interface::SyntaxPort;
use super::super::types::SourceText;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;
use ryo::languages::SourcePath;

/// What the language's tests stand in for `syntax` with: it reads any text as a stylesheet holding one comment, whose text is the
/// whole file.
pub(crate) struct SyntaxFake;

impl SyntaxPort for SyntaxFake {
    fn read(
        &self,
        text: SourceText<'_>,
        _at: SourcePath<'_>,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        let start = languages::Offset(0);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Stylesheet)), start);
        languages::decoded_token(
            builder,
            kind_bits(Kind::Extra(Extra::Comment)),
            start,
            end,
            languages::NameText(std::str::from_utf8(text.0).unwrap_or_default()),
        );
        languages::leave_node(builder, end);
    }

    fn walk(
        &self,
        text: SourceText<'_>,
        _at: SourcePath<'_>,
        builder: &mut languages::NodeTableBuilder<'_>,
    ) {
        use languages::{FieldCode, node_table};
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        let start = languages::Offset(0);
        let none = FieldCode(0);
        node_table::enter_node(builder, kind_bits(Kind::Node(Node::Stylesheet)), none, start);
        node_table::decoded_token(
            builder,
            kind_bits(Kind::Extra(Extra::Comment)),
            none,
            start,
            end,
            languages::NameText(std::str::from_utf8(text.0).unwrap_or_default()),
        );
        node_table::leave_node(builder, end);
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
