//! The `names` service's cases: the names a declaration binds and of what kind, and whether each name is read where it occurs.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::kind,
    types::{Goal, Kind},
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
        name: CaseName("a_destructuring_declarator_binds_its_names_not_its_keys"),
        word: Word::Succeeds,
        input: || "const { a, b: c = 1 } = o;",
        expect: Expect::Value(
            "Constant declares Node(Identifier)@8,Node(Identifier)@14 | Node(Identifier)@8:Binding Node(Identifier)@8:Binding Node(Identifier)@11:Binding Node(Identifier)@14:Binding Node(Identifier)@24:Use",
        ),
    },
    Case {
        name: CaseName("a_function_binds_its_name_and_reads_its_parameter"),
        word: Word::Succeeds,
        input: || "function f(x) { return x; }",
        expect: Expect::Value(
            "Function declares Node(Identifier)@9 | Node(Identifier)@9:Binding Node(Identifier)@11:Binding Node(Identifier)@23:Use",
        ),
    },
    Case {
        name: CaseName("a_class_binds_its_name_and_its_method_key"),
        word: Word::Succeeds,
        input: || "class K { m() {} }",
        expect: Expect::Value(
            "Class declares Node(Identifier)@6 | Node(Identifier)@6:Binding Node(Identifier)@10:Binding",
        ),
    },
    Case {
        name: CaseName("an_import_binds_its_local_name"),
        word: Word::Succeeds,
        input: || "import d from \"m\";",
        expect: Expect::Value("Import declares Node(Identifier)@7 | Node(Identifier)@7:Binding"),
    },
];

/// An element written out: its kind and where it starts.
fn at(tree: &languages::Tree<'_>, element: languages::Position) -> String {
    format!(
        "{:?}@{}",
        kind(tree, element),
        languages::start_at(tree.elements, element).0
    )
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
        Goal::Module,
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let tree = &tree;
    let answer: String = {
        let positions: Vec<languages::Position> = languages::positions(tree.elements).collect();
        let declared = positions
            .iter()
            .copied()
            .find(|at| {
                matches!(
                    kind(tree, *at),
                    Kind::Node(
                        Node::VariableDeclarator
                            | Node::FunctionDeclaration
                            | Node::ClassDeclaration
                            | Node::ImportDefaultSpecifier
                    )
                )
            })
            .map(|declaration| {
                let names: Vec<String> = super::service::declared(tree, declaration)
                    .map(|name| at(tree, name))
                    .collect();
                format!(
                    "{:?} declares {}",
                    super::service::declaration_kind(tree, declaration),
                    names.join(",")
                )
            })
            .unwrap_or_default();
        let read: Vec<String> = positions
            .iter()
            .copied()
            .filter(|each| kind(tree, *each) == Kind::Node(Node::Identifier))
            .map(|name| {
                format!(
                    "{}:{:?}",
                    at(tree, name),
                    super::service::referenced(tree, name)
                )
            })
            .collect();
        format!("{declared} | {}", read.join(" "))
    };
    Ok(Box::leak(answer.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
