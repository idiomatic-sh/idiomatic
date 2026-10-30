//! The `names` service's cases: the reasons are declared in their order, and the dotted names and import paths it makes.

use super::service::{dotted, unknown};
use super::types::Unresolved;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::imports::service::import_target;
use crate::domain::language::constants::UNKNOWN_REASONS;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};
use strum::IntoEnumIterator as _;

/// A source.
type Input = &'static str;
/// The dotted names of its attribute and name expressions that are no part of another, and the targets of its imports.
type Value = (&'static [&'static str], &'static [&'static str]);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_attribute_chain_is_one_dotted_name"),
        word: Word::Succeeds,
        input: || "a.b.c\n",
        expect: Expect::Value((&["a.b.c"], &[])),
    },
    Case {
        name: CaseName("relative_imports_keep_their_dots"),
        word: Word::Succeeds,
        input: || "from . import x\nfrom ...m.n import y\n",
        expect: Expect::Value((&[], &[".x", "...m.n.y"])),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let reasons: Vec<&'static str> = Unresolved::iter().map(<&'static str>::from).collect();
    let declared: Vec<&'static str> = UNKNOWN_REASONS.iter().map(|each| each.text.0).collect();
    let numbered = Unresolved::iter()
        .zip(UNKNOWN_REASONS)
        .all(|(reason, each)| unknown(reason) == languages::unknown_symbol(each.reason));
    if !numbered
        || reasons != declared
        || unknown(Unresolved::WildcardImport) == unknown(Unresolved::StarBase)
    {
        return Err(());
    }
    let mut file = parsed(source.as_bytes()).expect("the row's source parses");
    let elements = std::mem::take(&mut file.elements);
    let tree = Tree {
        elements: &elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let text = |symbols: &languages::Symbols, symbol| {
        languages::text_of(symbols, tree.source, symbol)
            .map_or(String::new(), |each| each.0.to_owned())
    };
    let mut names = Vec::new();
    let mut imports = Vec::new();
    for at in languages::positions(&elements) {
        if kind(&tree, at) == Kind::Node(Node::Attribute)
            && kind(&tree, languages::parent_at(&elements, at)) != Kind::Node(Node::Attribute)
            && let Some(name) = dotted(&tree, &mut file.symbols, at)
        {
            names.push(text(&file.symbols, name));
        }
        if kind(&tree, at) == Kind::Node(Node::Alias)
            && let Some(target) = import_target(&tree, &mut file.symbols, at)
        {
            imports.push(text(&file.symbols, target));
        }
    }
    let leak = |list: Vec<String>| -> &'static [&'static str] {
        let leaked: Vec<&'static str> = list
            .into_iter()
            .map(|each| &*Box::leak(each.into_boxed_str()))
            .collect();
        Box::leak(leaked.into_boxed_slice())
    };
    Ok((leak(names), leak(imports)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
