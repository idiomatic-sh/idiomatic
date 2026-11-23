//! Physical source locations of Python lexical units.
use super::{ColumnFile, Kind, TypedExtra};
use crate::domain::{syntax::service::kind_bits, tokens::types::Token};
use ryo::languages::{Position, node_table};

impl ColumnFile<'_, '_> {
    /// Each physical Python comment starts at a comment token, including a comment inside an interpolation.
    pub fn comment_positions(&self) -> &[Position] {
        node_table::ids_of_kind(self.table, kind_bits(Kind::Token(Token::Comment)))
    }

    /// Physical newlines can occur inside a string spelling or escaped by a backslash between tokens.
    pub fn visit_multiline_lexemes(
        &self,
        src: &[u8],
        mut visit: impl FnMut(Position, &'static str),
    ) {
        for (kind, form) in [
            (Kind::Token(Token::String), "string"),
            (Kind::Extra(TypedExtra::FStringPart), "formatted-string"),
            (Kind::Extra(TypedExtra::TStringPart), "template-string"),
            (Kind::Extra(TypedExtra::Whitespace), "explicit-join"),
        ] {
            for &id in node_table::ids_of_kind(self.table, kind_bits(kind)) {
                let span = node_table::span_of(self.table, id);
                let Some(bytes) = src.get(span.start.0 as usize..span.end.0 as usize) else {
                    continue;
                };
                if !bytes.iter().any(|b| matches!(b, b'\n' | b'\r')) {
                    continue;
                }
                if form == "explicit-join"
                    && !bytes
                        .windows(2)
                        .any(|p| p[0] == b'\\' && matches!(p[1], b'\n' | b'\r'))
                {
                    continue;
                }
                let form = if form == "string" {
                    let quoted = bytes
                        .iter()
                        .position(|b| matches!(b, 34 | 39))
                        .unwrap_or(bytes.len());
                    match bytes.get(quoted..quoted + 3) {
                        Some([34, 34, 34] | [39, 39, 39]) => "long-string",
                        _ => "short-string",
                    }
                } else {
                    form
                };
                visit(id, form);
            }
        }
    }

    /// A NEWLINE terminates a Python logical line; NL also marks a nonterminating tokenized line break.
    /// For a comment record the adjacent breaks and the first break after its next nontrivia token.
    pub fn visit_comment_breaks(
        &self,
        mut emit: impl FnMut(Position, u32, Option<u32>, Option<u32>),
    ) {
        use crate::domain::syntax::service::kind_of;
        let comments = self.comment_positions();
        if comments.is_empty() {
            return;
        }
        let own_end = node_table::own_end(self.table).0;
        let mut logical = Vec::new();
        let mut breaks = Vec::new();
        for token in [Token::Newline, Token::Nl] {
            for &id in node_table::ids_of_kind(self.table, kind_bits(Kind::Token(token))) {
                let s = node_table::span_of(self.table, id);
                if id.0 >= own_end || s.start == s.end {
                    continue;
                }
                breaks.push((s.start.0, s.end.0));
                if token == Token::Newline {
                    logical.push(s.start.0);
                }
            }
        }
        breaks.sort_unstable();
        // Consecutive comments share the first code token after the comment run.
        let mut next_token = 0;
        for &id in comments {
            let c = node_table::span_of(self.table, id);
            let preceding = breaks.partition_point(|&(_, end)| end <= c.start.0);
            let last = preceding.checked_sub(1).map_or(0, |i| breaks[i].1);
            let logical_end = logical
                .get(logical.partition_point(|&at| at < c.end.0))
                .copied();
            next_token = next_token.max(id.0.saturating_add(1));
            while next_token < own_end {
                let at = Position(next_token);
                let span = node_table::span_of(self.table, at);
                let kind = kind_of(node_table::kind_of(self.table, at));
                if let Kind::Token(token) = kind
                    && !matches!(
                        token,
                        Token::Comment | Token::Nl | Token::Newline | Token::Indent | Token::Dedent
                    )
                    && span.end > span.start
                    && span.start >= c.end
                {
                    break;
                }
                next_token += 1;
            }
            let next = (next_token < own_end).then_some(Position(next_token));
            let following = next.and_then(|at| {
                let end = node_table::span_of(self.table, at).end.0;
                breaks
                    .get(breaks.partition_point(|&(begin, _)| begin < end))
                    .map(|v| v.0)
            });
            emit(id, last, logical_end, following);
        }
    }
}
