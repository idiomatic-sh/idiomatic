//! The `syntax` service's cases: the tree of a file, its kinds by the standards' names and the names it writes.

use super::service::{kind, kind_name, kind_symbol, kinds, name_of};
use super::types::Kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// A source.
type Input = &'static str;
/// The name of the kind of each node, in order, and the names the name expressions write.
type Value = (&'static [&'static str], &'static [&'static str]);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_call_is_named_by_the_standard"),
    word: Word::Succeeds,
    input: || "f(x)\n",
    expect: Expect::Value((
        &["module", "expr", "call", "name", "call-arguments", "name"],
        &["f", "x"],
    )),
}];

/// Whether every kind of the language, nodes, tokens and extras together, has a name of its own.
const NAMING: &[Case<(), bool, Failure>] = &[Case {
    name: CaseName("every_kind_is_named_once"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

fn named_once(_: ()) -> bool {
    let mut names: Vec<&str> = kinds().map(|each| kind_name(each).0).collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    names.len() == count
}

fn subject(source: Input) -> Value {
    let mut file = parsed(source.as_bytes()).expect("the row's source parses");
    let elements = std::mem::take(&mut file.elements);
    let tree = Tree {
        elements: &elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut kinds = Vec::new();
    let mut names = Vec::new();
    for at in languages::positions(&elements) {
        if languages::flags_at(&elements, at).0 & languages::TOKEN.0 != 0 {
            continue;
        }
        let symbol = kind_symbol(&tree, &mut file.symbols, at);
        let text = languages::text_of(&file.symbols, tree.source, symbol)
            .map_or(String::new(), |each| each.0.to_owned());
        kinds.push(&*Box::leak(text.into_boxed_str()));
        if kind(&tree, at) == Kind::Node(Node::Name)
            && let Some(name) = name_of(&tree, at)
        {
            let spelled = languages::text_of(&file.symbols, tree.source, name)
                .map_or(String::new(), |each| each.0.to_owned());
            names.push(&*Box::leak(spelled.into_boxed_str()));
        }
    }
    (
        Box::leak(kinds.into_boxed_slice()),
        Box::leak(names.into_boxed_slice()),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
    run(NAMING, Word::Succeeds, |input| Ok(named_once(input)));
}
