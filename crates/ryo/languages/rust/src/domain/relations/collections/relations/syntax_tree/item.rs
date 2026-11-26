//! The `syntax_tree` relation of the language: every node and every token, whitespace and comments included.

use crate::domain::model::types::ReadIndexes;
use crate::domain::syntax::service::{kind_symbol, token_text};
use ryo::languages;

/// The reader the `fills_rows!` below binds the `syntax_tree` relation's rows to.
pub(crate) struct SyntaxTreeItem;
