//! The `targets` service's cases: what an import imports, what a url points at, and what each belongs to.

use super::service::{import_token, owner, reference, url_token};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// A stylesheet.
type Input = &'static str;
/// For each element the service finds a target of, in order: its kind, the kind of the token the adapter wrote the target on, and
/// the kind of what the element belongs to.
type Value = &'static [(Kind, Kind, Kind)];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_import_imports_its_string_or_its_url"),
        word: Word::Succeeds,
        input: || "@import \"a.css\"; @import url(b.css); @import url(\"c.css\");",
        expect: Expect::Value(&[
            (
                Kind::Node(Node::AtRule),
                Kind::Token(Token::AtKeyword),
                Kind::Node(Node::Stylesheet),
            ),
            (
                Kind::Node(Node::AtRule),
                Kind::Token(Token::AtKeyword),
                Kind::Node(Node::Stylesheet),
            ),
            (
                Kind::Node(Node::AtRule),
                Kind::Token(Token::AtKeyword),
                Kind::Node(Node::Stylesheet),
            ),
        ]),
    },
    Case {
        name: CaseName("a_url_in_a_declaration_belongs_to_it"),
        word: Word::Succeeds,
        input: || "a { b: url(c.png) url(\"d.png\") }",
        expect: Expect::Value(&[
            (
                Kind::Token(Token::Url),
                Kind::Token(Token::Url),
                Kind::Node(Node::Declaration),
            ),
            (
                Kind::Node(Node::Function),
                Kind::Token(Token::Function),
                Kind::Node(Node::Declaration),
            ),
        ]),
    },
    Case {
        name: CaseName("a_namespace_url_is_nothing_and_an_import_url_is_its_import"),
        word: Word::Succeeds,
        input: || "@namespace url(a); @import url(b);",
        expect: Expect::Value(&[(
            Kind::Node(Node::AtRule),
            Kind::Token(Token::AtKeyword),
            Kind::Node(Node::Stylesheet),
        )]),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let file = parsed(source.as_bytes()).map_err(drop)?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let found: Vec<(Kind, Kind, Kind)> = languages::positions(&file.elements)
        .filter_map(|at| {
            let found = import_token(&tree, at).or_else(|| {
                let token = url_token(&tree, at)?;
                reference(&tree, at).map(|_| token)
            })?;
            Some((
                kind(&tree, at),
                kind(&tree, found),
                kind(&tree, owner(&tree, at)),
            ))
        })
        .collect();
    Ok(Box::leak(found.into_boxed_slice()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
