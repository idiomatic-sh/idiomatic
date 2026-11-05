//! The `static_import` relation of the language: every `@import`, by the path of the run it imports (the adapter resolves the url the
//! library decoded against the stylesheet's own path). An `@import`
//! brings in every rule of the stylesheet it names and binds no name it could list: its binding is the unknown value of that.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{decoded, kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::targets::service::{import_token, owner};
use ryo::languages;

/// The reader the `parse` contract hands the `static_import` relation to, which reads its rows from a file's tree.
pub(crate) struct StaticImportItem;

/// The number of `imports-every-rule` in the language's list of reasons, `UNKNOWN_REASONS`.
const EVERY_RULE: languages::UnknownReason = languages::UnknownReason(48);

ryo::reads! {
    StaticImportItem => languages::StaticImport |tree, at, symbols| {
        kinds: [Kind::Node(Node::AtRule)] as kind_bits,
        given token = import_token(tree, at)?;
        given target = decoded(tree, symbols, token)?;
        row {
            occurrence: node(at),
            owner: node(owner(tree, at)),
            binding: value(languages::unknown_symbol(EVERY_RULE)),
            target: value(target),
            file: file,
            span: span(at),
        }
    }
}
