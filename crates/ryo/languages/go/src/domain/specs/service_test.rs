//! The `specs` service's cases: the names and the type of what a declaration lists, and what the declaration declares.

use super::service::{
    declared_type, declares, import_name, import_path, name, names, underlying, value_type,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::{asked, of_kind, spans};

/// What a row asks of the first spec of the kind in the source.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The names of a constant or variable spec.
    Names,
    /// The type it gives them.
    ValueType,
    /// What the declaration that holds the spec declares.
    Declares(GoNode),
    /// What the type spec declares.
    Underlying,
    /// The name of a type spec.
    TypeName,
    /// The type a type spec declares.
    DeclaredType,
    /// The name an import gives the package, and its path.
    Import,
}

/// A source and the question about its first spec.
type Input = (&'static str, Question);
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_value_spec_declares_the_names_before_its_type_and_its_values"),
        word: Word::Succeeds,
        input: || ("package p\nvar a, b int = 1, 2\n", Question::Names),
        expect: Expect::Value("a b"),
    },
    Case {
        name: CaseName("a_value_spec_gives_its_names_the_type_before_the_equal_sign"),
        word: Word::Succeeds,
        input: || ("package p\nvar a, b int = 1, 2\n", Question::ValueType),
        expect: Expect::Value("int"),
    },
    Case {
        name: CaseName("a_value_spec_with_no_type_gives_its_names_none"),
        word: Word::Succeeds,
        input: || ("package p\nvar c = 3\n", Question::ValueType),
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("a_const_declaration_declares_constants"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nconst c = 1\n",
                Question::Declares(GoNode::ValueSpec),
            )
        },
        expect: Expect::Value("Constants"),
    },
    Case {
        name: CaseName("a_var_declaration_declares_variables"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nvar v int\n",
                Question::Declares(GoNode::ValueSpec),
            )
        },
        expect: Expect::Value("Variables"),
    },
    Case {
        name: CaseName("a_type_declaration_declares_types"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\ntype T int\n",
                Question::Declares(GoNode::TypeSpec),
            )
        },
        expect: Expect::Value("Types"),
    },
    Case {
        name: CaseName("an_import_declaration_imports"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nimport \"a\"\n",
                Question::Declares(GoNode::ImportSpec),
            )
        },
        expect: Expect::Value("Imports"),
    },
    Case {
        name: CaseName("a_struct_type_declares_a_structure"),
        word: Word::Succeeds,
        input: || ("package p\ntype T struct{}\n", Question::Underlying),
        expect: Expect::Value("Structure"),
    },
    Case {
        name: CaseName("an_interface_type_declares_an_interface"),
        word: Word::Succeeds,
        input: || ("package p\ntype T interface{}\n", Question::Underlying),
        expect: Expect::Value("Interface"),
    },
    Case {
        name: CaseName("a_type_declared_with_an_equal_sign_is_an_alias"),
        word: Word::Succeeds,
        input: || ("package p\ntype A = T\n", Question::Underlying),
        expect: Expect::Value("Alias"),
    },
    Case {
        name: CaseName("any_other_type_is_another_type"),
        word: Word::Succeeds,
        input: || ("package p\ntype ID int\n", Question::Underlying),
        expect: Expect::Value("Other"),
    },
    Case {
        name: CaseName("a_type_spec_is_named_by_its_first_element"),
        word: Word::Succeeds,
        input: || ("package p\ntype ID int\n", Question::TypeName),
        expect: Expect::Value("ID"),
    },
    Case {
        name: CaseName("a_type_spec_declares_its_last_element"),
        word: Word::Succeeds,
        input: || ("package p\ntype ID int\n", Question::DeclaredType),
        expect: Expect::Value("int"),
    },
    Case {
        name: CaseName("an_import_gives_the_package_the_name_before_its_path"),
        word: Word::Succeeds,
        input: || ("package p\nimport str \"strings\"\n", Question::Import),
        expect: Expect::Value("str \"strings\""),
    },
    Case {
        name: CaseName("an_import_with_no_name_has_only_a_path"),
        word: Word::Succeeds,
        input: || ("package p\nimport \"strings\"\n", Question::Import),
        expect: Expect::Value("none \"strings\""),
    },
];

fn answer((source, question): Input) -> String {
    asked(source, |tree| {
        let spec_of = |node: GoNode| of_kind(tree, Kind::Node(node)).first().copied();
        let shown = |at: Option<ryo::languages::Position>| {
            at.map_or_else(|| "none".to_owned(), |each| spans(tree, each))
        };
        match question {
            Question::Names => spec_of(GoNode::ValueSpec)
                .map(|spec| {
                    names(tree, spec)
                        .map(|each| spans(tree, each))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default(),
            Question::ValueType => {
                shown(spec_of(GoNode::ValueSpec).and_then(|spec| value_type(tree, spec)))
            }
            Question::Declares(node) => spec_of(node)
                .and_then(|spec| declares(tree, spec))
                .map_or_else(|| "none".to_owned(), |each| format!("{each:?}")),
            Question::Underlying => spec_of(GoNode::TypeSpec)
                .map(|spec| format!("{:?}", underlying(tree, spec)))
                .unwrap_or_default(),
            Question::TypeName => shown(spec_of(GoNode::TypeSpec).and_then(|spec| name(tree, spec))),
            Question::DeclaredType => {
                shown(spec_of(GoNode::TypeSpec).and_then(|spec| declared_type(tree, spec)))
            }
            Question::Import => spec_of(GoNode::ImportSpec)
                .map(|spec| {
                    format!(
                        "{} {}",
                        shown(import_name(tree, spec)),
                        shown(import_path(tree, spec))
                    )
                })
                .unwrap_or_default(),
        }
    })
}

fn subject(input: Input) -> Value {
    Box::leak(answer(input).into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
