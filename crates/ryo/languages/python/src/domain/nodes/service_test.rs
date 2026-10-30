//! The `nodes` service's cases: every constructor of the pinned standard is a node, named in kebab case, and each node is in the
//! category its ASDL type says.

use super::service::{category, standard};
use super::types::{Category, Node};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Kind;
use strum::IntoEnumIterator as _;

/// The standard, pinned: the ASDL of `CPython`'s `ast`.
const ASDL: &str = include_str!("../../../tests/fixtures/standard/Python.asdl");

/// The page of the pinned standards, which names each standard and its version.
const PINNED: &str = include_str!("../../../tests/fixtures/standard/README.md");

/// What a row asks: the constructors of the standard no node names, or the category of one node.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Every constructor of the ASDL, by its name.
    Standard(&'static str),
    /// One node.
    Node(Node),
}

type Input = Given;
/// The constructors no node names; or the node's name and category.
type Value = Seen;
type Failure = ();

/// What a row looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The constructors of the standard that are no node.
    Missing(&'static [&'static str]),
    /// A node's name and category.
    Named(&'static str, Category),
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_constructor_of_the_standard_is_a_node"),
        word: Word::Succeeds,
        input: || Given::Standard(ASDL),
        expect: Expect::Value(Seen::Missing(&[])),
    },
    Case {
        name: CaseName("a_definition_is_a_statement_named_in_kebab_case"),
        word: Word::Succeeds,
        input: || Given::Node(Node::FunctionDef),
        expect: Expect::Value(Seen::Named("function-def", Category::Statement)),
    },
    Case {
        name: CaseName("a_product_type_is_a_part_named_as_the_standard_writes_it"),
        word: Word::Succeeds,
        input: || Given::Node(Node::MatchCase),
        expect: Expect::Value(Seen::Named("match-case", Category::Part)),
    },
];

/// The constructors of an ASDL, the capitalized names after `=` or `|`, in kebab case as the nodes are named: a product type is
/// named by its type (`arguments`, `arg`) and is no constructor.
fn constructors(asdl: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in asdl.lines() {
        let code = line.split("--").next().unwrap_or_default();
        for (at, _) in code.match_indices(['=', '|']) {
            let rest = code.get(at + 1..).unwrap_or_default().trim_start();
            let constructor = rest
                .split(|each: char| !each.is_ascii_alphanumeric())
                .next()
                .unwrap_or_default();
            if constructor.starts_with(|each: char| each.is_ascii_uppercase()) {
                let mut kebab = String::new();
                for (place, letter) in constructor.char_indices() {
                    if letter.is_ascii_uppercase() && place > 0 {
                        kebab.push('-');
                    }
                    kebab.push(letter.to_ascii_lowercase());
                }
                found.push(kebab);
            }
        }
    }
    found
}

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Standard(asdl) => {
            let named: Vec<&str> = Node::iter()
                .map(|node| kind_name(Kind::Node(node)).0)
                .collect();
            let missing: Vec<&'static str> = constructors(asdl)
                .into_iter()
                .filter(|each| !named.contains(&each.as_str()))
                .map(|each| &*Box::leak(each.into_boxed_str()))
                .collect();
            if !PINNED.contains(standard().0) {
                return Err(());
            }
            Ok(Seen::Missing(Box::leak(missing.into_boxed_slice())))
        }
        Given::Node(node) => Ok(Seen::Named(kind_name(Kind::Node(node)).0, category(node))),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
