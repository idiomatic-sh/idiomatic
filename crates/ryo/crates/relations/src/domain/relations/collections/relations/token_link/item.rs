//! Optional edges over existing physical native lexer-token identities.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// One physical lexical successor, matched delimiter, or enclosing delimiter edge.
#[derive(Clone, Copy, Debug)]
pub struct TokenLink;
impl super::super::interface::sealed::Sealed for TokenLink {}

/// Declare optional token metadata without a second token population.
#[macro_export]
#[doc(hidden)]
macro_rules! token_link {
    ($callback:path) => {
        $callback! {
            item TokenLink,
            row { /// One directed lexical edge.
                TokenLinkRow },
            column { /// Typed token-link columns.
                TokenLinkColumn },
            expected { /// Independently expected token links.
                TokenLinkExpectedRow },
            class Lexical,
            scope File,
            notes Unneeded,
            columns {
                /// Existing native source token identity.
                source Source: NodeId => Node;
                /// Existing native target token identity; absent edges have no row.
                target Target: NodeId => Node;
                /// next-token, matching-delimiter (opener to closer), or enclosing-delimiter (token to opener).
                link_kind LinkKind: Symbol => Symbol;
                /// Containing file.
                file File: FileId => File;
                /// Source-token span, not a synthesized combined interval.
                span Span: Span => Span;
            }
        }
    };
}
crate::token_link!(crate::relation_row);
