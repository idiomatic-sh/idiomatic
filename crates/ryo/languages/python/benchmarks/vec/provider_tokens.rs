//! redo-general (THROWAWAY, bead ryo-fmx3.1): the PROVIDER's lexical model (Python's): which kinds are tokens and carry
//! their source text, the delimiter pairs, the error token that breaks the order, and from them the `token-link` fact: the
//! token order of a file with its links (next token, matching delimiter, enclosing delimiter). Moved here from the
//! executor (`finish.rs` `toks`, `token_links`, `is_token`; `finisher.rs` and `bench.rs` kind names; `facts.rs`
//! `TokenKinds`; leak review 1.5, 6.5): one definition, read by the executor and by the first half's store through
//! `finish::TokenOrder`, so neither holds a kind name, a kind range or a pairing rule.
//!
//! Why a provider fact and not a declaration of token classes the executor reads: a declaration would leave the executor
//! the pairing rule (an error token resets the order, an unmatched closer drops the open delimiters), which is the
//! language's lexical model; as a fact the executor holds only the relation's catalog shape (source, target, link kind).

use super::finish::{TokenEvent, TokenOrder};
use ryo::languages::{NodeTable, Position, node_table};

/// Python's tokens as the node table codes them.
pub(super) struct Lexicon {
    /// Token kinds are the codes `first..=last` (the provider's vocabulary orders them so).
    first: u16,
    last: u16,
    error: u16,
    pairs: [(u16, u16); 3],
}

impl Lexicon {
    /// From the provider's code of each kind name.
    pub(super) fn new(kind: &dyn Fn(&str) -> Option<u16>) -> Self {
        let k = |n: &str| kind(n).unwrap_or_else(|| panic!("the provider's kind {n}"));
        Self { first: k("name-token"), last: k("colonequal"), error: k("errortoken"), pairs: [(k("lpar"), k("rpar")), (k("lsqb"), k("rsqb")), (k("lbrace"), k("rbrace"))] }
    }
    fn is_token(&self, k: u16) -> bool {
        k >= self.first && k <= self.last
    }
    fn closer(&self, k: u16) -> Option<u16> {
        self.pairs.iter().find(|(o, _)| *o == k).map(|(_, c)| *c)
    }
    fn is_closer(&self, k: u16) -> bool {
        self.pairs.iter().any(|(_, c)| *c == k)
    }
}

impl TokenOrder for Lexicon {
    fn has_text(&self, kind: u16) -> bool {
        self.is_token(kind) || kind == self.error
    }

    fn links(&self, table: &NodeTable, each: &mut dyn FnMut(TokenEvent)) {
        let start = node_table::node_count(table).0 as u32;
        let end = node_table::own_end(table).0;
        let mut stack: Vec<(u16, u32)> = Vec::new();
        let mut previous: Option<u32> = None;
        for at in start..end {
            let k = node_table::kind_of(table, Position(at)).0;
            if !self.is_token(k) {
                continue;
            }
            // an error token breaks the order: no link crosses it
            if k == self.error {
                previous = None;
                stack.clear();
                continue;
            }
            each(TokenEvent::Token(at));
            let mut paired = None;
            let mut enclosing = stack.last().map(|o| o.1);
            if let Some(close) = self.closer(k) {
                stack.push((close, at));
            } else if self.is_closer(k) {
                match stack.pop() {
                    Some((close, open)) if close == k => {
                        paired = Some(open);
                        enclosing = stack.last().map(|o| o.1);
                    }
                    // an unmatched closer drops every open delimiter
                    _ => {
                        stack.clear();
                        enclosing = None;
                    }
                }
            }
            if let Some(p) = previous.replace(at) {
                each(TokenEvent::Next(p, at));
            }
            if let Some(o) = paired {
                each(TokenEvent::Matching(o, at));
            }
            if let Some(e) = enclosing {
                each(TokenEvent::Enclosing(at, e));
            }
        }
    }
}
