//! Linear links over the completed native token prefix, never a second lexer.
use super::types::{Event, TokenLink, LinkKind, Scan};
use crate::domain::{
    syntax::{service::kind, types::Kind},
    tokens::types::Token,
};
use ryo::languages::{self, Position, Tree};

/// Native root extent excludes separately adopted embedded expressions.
pub(crate) fn scan<'tree, 'source>(tree: &'tree Tree<'source>) -> Scan<'tree, 'source> {
    let end = if languages::end_of(tree.elements).0 == 0 {
        0
    } else {
        languages::last_at(tree.elements, Position(0))
            .0
            .saturating_add(1)
    };
    Scan {
        tree,
        positions: 0..end,
        previous: None,
        stack: Vec::new(),
        pending: [None; 2],
        incomplete: false,
        reported: false,
    }
}

/// Public vocabulary belongs to the structural fact, not a linter.
pub(crate) const fn link_kind(kind: LinkKind) -> &'static str {
    match kind {
        LinkKind::Next => "next-token",
        LinkKind::Matching => "matching-delimiter",
        LinkKind::Enclosing => "enclosing-delimiter",
    }
}

fn closing(open: Token) -> Option<Token> {
    match open {
        Token::Lpar => Some(Token::Rpar),
        Token::Lsqb => Some(Token::Rsqb),
        Token::Lbrace => Some(Token::Rbrace),
        _ => None,
    }
}

impl Iterator for Scan<'_, '_> {
    type Item = Event;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(link) = self.pending[0].take() {
            self.pending[0] = self.pending[1].take();
            return Some(Event::Link(link));
        }
        for raw in self.positions.by_ref() {
            let at = Position(raw);
            let Kind::Token(token) = kind(self.tree, at) else {
                continue;
            };
            if token == Token::Errortoken {
                self.incomplete = true;
                self.previous = None;
                self.stack.clear();
                continue;
            }
            let mut paired = None;
            let mut enclosing = self.stack.last().map(|(_, opener)| *opener);
            if closing(token).is_some() {
                if self.stack.try_reserve(1).is_err() {
                    self.incomplete = true;
                    self.previous = None;
                    self.stack.clear();
                    continue;
                }
                self.stack.push((token, at));
            } else if matches!(token, Token::Rpar | Token::Rsqb | Token::Rbrace) {
                match self.stack.pop() {
                    Some((open, source)) if closing(open) == Some(token) => {
                        paired = Some(TokenLink {
                            source,
                            target: at,
                            kind: LinkKind::Matching,
                        });
                        enclosing = self.stack.last().map(|(_, opener)| *opener);
                    }
                    Some(_) | None => {
                        self.incomplete = true;
                        self.stack.clear();
                        enclosing = None;
                    }
                }
            }
            let enclosed = enclosing.map(|target| TokenLink {
                source: at,
                target,
                kind: LinkKind::Enclosing,
            });
            if let Some(source) = self.previous.replace(at) {
                self.pending = [paired.or(enclosed), paired.and(enclosed)];
                return Some(Event::Link(TokenLink {
                    source,
                    target: at,
                    kind: LinkKind::Next,
                }));
            }
            if let Some(link) = paired {
                self.pending[0] = enclosed;
                return Some(Event::Link(link));
            }
            if let Some(link) = enclosed {
                return Some(Event::Link(link));
            }
        }
        if !self.stack.is_empty() {
            self.incomplete = true;
            self.stack.clear();
        }
        if self.incomplete && !self.reported {
            self.reported = true;
            return Some(Event::Incomplete);
        }
        None
    }
}
