//! Linear links over the completed native token prefix, never a second lexer.
use super::constants;
use super::types::{Coverage, Event, LinkKind, Opener, Pairing, PositionCursor, Scan, TokenLink};
use crate::domain::{
    syntax::{service::kind, types::Kind},
    tokens::types::Token,
};
use ryo::languages::{self, NameText, Position, Tree};

/// Native root extent excludes separately adopted embedded expressions.
pub(crate) fn scan<'tree, 'source>(
    tree: &'tree Tree<'source>,
) -> impl Iterator<Item = Event> + use<'tree, 'source> {
    let end = if languages::end_of(tree.elements).0 == 0 {
        0
    } else {
        languages::last_at(tree.elements, Position(0))
            .0
            .saturating_add(1)
    };
    let mut state = Scan {
        tree,
        positions: PositionCursor {
            next: Position(0),
            end: Position(end),
        },
        previous: None,
        stack: Vec::new(),
        pending: [None; 2],
        pairing: Pairing::Whole,
        reported: Coverage::Pending,
    };
    std::iter::from_fn(move || next_event(&mut state))
}

/// Public vocabulary belongs to the structural fact, not a linter.
pub(crate) const fn link_kind(kind: LinkKind) -> NameText<'static> {
    match kind {
        LinkKind::Next => NameText(constants::NEXT_TOKEN),
        LinkKind::Matching => NameText(constants::MATCHING_DELIMITER),
        LinkKind::Enclosing => NameText(constants::ENCLOSING_DELIMITER),
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

/// The next link or coverage event of the scan, in source order.
fn next_event(scan: &mut Scan<'_, '_>) -> Option<Event> {
    if let Some(link) = scan.pending[0].take() {
        scan.pending[0] = scan.pending[1].take();
        return Some(Event::Link(link));
    }
    while scan.positions.next.0 < scan.positions.end.0 {
        let at = scan.positions.next;
        scan.positions.next = Position(at.0.saturating_add(1));
        let Kind::Token(token) = kind(scan.tree, at) else {
            continue;
        };
        if token == Token::Errortoken {
            scan.pairing = Pairing::Lost;
            scan.previous = None;
            scan.stack.clear();
            continue;
        }
        let mut paired = None;
        let mut enclosing = scan.stack.last().map(|opener| opener.at);
        if closing(token).is_some() {
            if scan.stack.try_reserve(1).is_err() {
                scan.pairing = Pairing::Lost;
                scan.previous = None;
                scan.stack.clear();
                continue;
            }
            scan.stack.push(Opener { token, at });
        } else if matches!(token, Token::Rpar | Token::Rsqb | Token::Rbrace) {
            match scan.stack.pop() {
                Some(opener) if closing(opener.token) == Some(token) => {
                    paired = Some(TokenLink {
                        source: opener.at,
                        target: at,
                        kind: LinkKind::Matching,
                    });
                    enclosing = scan.stack.last().map(|opener| opener.at);
                }
                Some(_) | None => {
                    scan.pairing = Pairing::Lost;
                    scan.stack.clear();
                    enclosing = None;
                }
            }
        }
        let enclosed = enclosing.map(|target| TokenLink {
            source: at,
            target,
            kind: LinkKind::Enclosing,
        });
        if let Some(source) = scan.previous.replace(at) {
            scan.pending = [paired.or(enclosed), paired.and(enclosed)];
            return Some(Event::Link(TokenLink {
                source,
                target: at,
                kind: LinkKind::Next,
            }));
        }
        if let Some(link) = paired {
            scan.pending[0] = enclosed;
            return Some(Event::Link(link));
        }
        if let Some(link) = enclosed {
            return Some(Event::Link(link));
        }
    }
    if !scan.stack.is_empty() {
        scan.pairing = Pairing::Lost;
        scan.stack.clear();
    }
    if scan.pairing == Pairing::Lost && scan.reported == Coverage::Pending {
        scan.reported = Coverage::Reported;
        return Some(Event::Incomplete);
    }
    None
}
