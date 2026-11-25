//! The fake of the `syntax` port: what the language's tests stand in for it with.

use super::super::interface::SyntaxPort;
use super::super::types::SourceText;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;

use crate::domain::syntax::service::numbered;
use crate::domain::syntax::types::{Output, Roling};
use ryo::languages;

/// What the language's tests stand in for `syntax` with: it reads any text as a file holding one token of text it could not read, the whole text.
pub(crate) struct SyntaxFake;

impl SyntaxPort for SyntaxFake {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        let start = languages::Offset(0);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::File)), start);
        languages::token(builder, kind_bits(Kind::Extra(Extra::Error)), start, end);
        languages::leave_node(builder, end);
    }

    fn walk(&self, text: SourceText<'_>, roling: &mut Roling, builder: &mut languages::NodeTableBuilder<'_>) {
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        numbered(roling, languages::ByteSlice(text.0), builder, |out: &mut dyn Output| {
            out.enter(Kind::Node(Node::File), languages::Offset(0));
            out.token(Kind::Extra(Extra::Error), languages::Offset(0), end, None);
            out.leave(end);
        });
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
