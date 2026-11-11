//! The `imports` service's cases: the bindings an import writes, the name each binds and the module it names.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::kind,
    types::{Dialect, Kind},
};
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;

/// The source a row parses.
type Input = &'static str;
/// What the service answered over it, written out.
type Value = &'static str;
/// Reading a tree is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("default_and_renamed_bindings"),
        word: Word::Succeeds,
        input: || "import a, { b as c } from \"m\";",
        expect: Expect::Value("a,c from m"),
    },
    Case {
        name: CaseName("a_side_effect_import_binds_the_unknown_name"),
        word: Word::Succeeds,
        input: || "import \"side\";",
        expect: Expect::Value("- from side"),
    },
    Case {
        name: CaseName("require_names_its_string_module"),
        word: Word::Succeeds,
        input: || "const x = require(\"r\");",
        expect: Expect::Value("- from r"),
    },
    Case {
        name: CaseName("require_of_a_computed_module_names_the_unknown_one"),
        word: Word::Succeeds,
        input: || "const y = require(name);",
        expect: Expect::Value("- from -"),
    },
];

/// The first element of the tree of kind `node`.
fn first(tree: &languages::Tree<'_>, node: Node) -> Option<languages::Position> {
    languages::positions(tree.elements).find(|at| kind(tree, *at) == Kind::Node(node))
}

/// The text of a name, or `-` for the unknown one.
fn text(
    tree: &languages::Tree<'_>,
    symbols: &languages::Symbols,
    name: languages::Symbol,
) -> String {
    languages::text_of(symbols, tree.source, name)
        .map_or_else(|| String::from("-"), |name| name.0.to_owned())
}

/// Parses the row's source and writes out what the service answers over its tree.
fn subject(source: Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let bytes = languages::ByteSlice(source.as_bytes());
    factory::oxc_parser().read(
        SourceText(source.as_bytes()),
        Dialect::TypeScript,
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let tree = &tree;
    let symbols = &mut symbols;
    let answer: String = {
        let at = first(tree, Node::ImportDeclaration)
            .or_else(|| first(tree, Node::CallExpression))
            .expect("the row imports");
        let names = super::service::imports(tree, at)
            .collect::<Vec<_>>()
            .into_iter()
            .map(|binding| {
                let name = super::service::import_name(tree, symbols, binding);
                text(tree, symbols, name)
            })
            .collect::<Vec<_>>()
            .join(",");
        let source = super::service::import_source(tree, symbols, at).map_or_else(
            || String::from("none"),
            |module| text(tree, symbols, module),
        );
        format!("{names} from {source}")
    };
    Ok(Box::leak(answer.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
