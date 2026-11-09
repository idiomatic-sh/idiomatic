//! The `syntax` port's fake makes the tree it was given, every element with its kind and the text it spans.

use super::SyntaxFake;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::fixtures::contract::{Decoding, ElementSource, Made, Want, made_by};
use crate::outbound::syntax::fixtures::script::{node, space, text_of, token};

#[test]
fn succeeds() {
    let tree = node(
        Kind::Node(Node::SourceFile),
        vec![node(
            Kind::Node(Node::Fn),
            vec![
                token(Kind::Token(Token::FnKw), "fn"),
                space(" "),
                node(
                    Kind::Node(Node::Name),
                    vec![token(Kind::Token(Token::Ident), "main")],
                ),
            ],
        )],
    );
    let made = made_by(&SyntaxFake(tree.clone()), &text_of(&tree));
    let want = |kind, text| Made {
        origin: ElementSource::File,
        element: Want { kind, text },
        value: Decoding::Plain,
    };
    assert_eq!(
        made,
        vec![
            want(Kind::Node(Node::SourceFile), "fn main"),
            want(Kind::Node(Node::Fn), "fn main"),
            want(Kind::Token(Token::FnKw), "fn"),
            want(Kind::Token(Token::Whitespace), " "),
            want(Kind::Node(Node::Name), "main"),
            want(Kind::Token(Token::Ident), "main"),
        ]
    );
}
