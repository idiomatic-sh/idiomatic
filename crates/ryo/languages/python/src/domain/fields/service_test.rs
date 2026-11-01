// ignore: idiomatic.arch/file-length - one row for each role of the vocabulary, and the check that every role has one: the table is the proof, and splitting it would split that check
//! The `fields` service's cases: the role of an edge of the tree under its parent, one row for each role of the vocabulary, on source
//! read through the parse. The roles are the fields of `Python.asdl`; the adapter's cases hold them to what `CPython`'s `ast.dump`
//! prints for every pinned file.

use super::service::role_of;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::labels::service::{field_name, fields};
use crate::domain::labels::types::Field;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::notes::factory::postcard;
use crate::outbound::syntax::factory::ruff_python_parser;
use ryo::languages::{self, Position, Tree};
use std::sync::Arc;

/// An edge of the tree of some source: the nth edge, in source order, from an element of a kind to an element of a kind.
#[derive(Clone, Copy)]
struct Edge {
    source: &'static str,
    parent: Kind,
    child: Kind,
    nth: usize,
}

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

const fn edge(source: &'static str, parent: Kind, child: Kind, nth: usize) -> Edge {
    Edge {
        source,
        parent,
        child,
        nth,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("none_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "pass\n",
                Kind::Node(Node::Pass),
                Kind::Token(Token::Name),
                0,
            )
        },
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("body_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "pass\n",
                Kind::Node(Node::Module),
                Kind::Node(Node::Pass),
                0,
            )
        },
        expect: Expect::Value("body"),
    },
    Case {
        name: CaseName("name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f(): pass\n",
                Kind::Node(Node::FunctionDef),
                Kind::Extra(Extra::Identifier),
                0,
            )
        },
        expect: Expect::Value("name"),
    },
    Case {
        name: CaseName("args_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f(x)\n",
                Kind::Extra(Extra::CallArguments),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("args"),
    },
    Case {
        name: CaseName("decorator_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "@d\ndef f(): pass\n",
                Kind::Node(Node::FunctionDef),
                Kind::Extra(Extra::Decorator),
                0,
            )
        },
        expect: Expect::Value("decorator-list"),
    },
    Case {
        name: CaseName("returns_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f() -> int: pass\n",
                Kind::Node(Node::FunctionDef),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("returns"),
    },
    Case {
        name: CaseName("type_params_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f[T](): pass\n",
                Kind::Node(Node::FunctionDef),
                Kind::Extra(Extra::TypeParams),
                0,
            )
        },
        expect: Expect::Value("type-params"),
    },
    Case {
        name: CaseName("bases_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class C(B): pass\n",
                Kind::Extra(Extra::CallArguments),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("bases"),
    },
    Case {
        name: CaseName("keywords_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f(k=1)\n",
                Kind::Extra(Extra::CallArguments),
                Kind::Node(Node::Keyword),
                0,
            )
        },
        expect: Expect::Value("keywords"),
    },
    Case {
        name: CaseName("value_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x = 1\n",
                Kind::Node(Node::Assign),
                Kind::Node(Node::Constant),
                0,
            )
        },
        expect: Expect::Value("value"),
    },
    Case {
        name: CaseName("targets_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x = 1\n",
                Kind::Node(Node::Assign),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("targets"),
    },
    Case {
        name: CaseName("target_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x += 1\n",
                Kind::Node(Node::AugAssign),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("target"),
    },
    Case {
        name: CaseName("op_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a + b\n",
                Kind::Node(Node::BinOp),
                Kind::Token(Token::Plus),
                0,
            )
        },
        expect: Expect::Value("op"),
    },
    Case {
        name: CaseName("annotation_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x: int\n",
                Kind::Node(Node::AnnAssign),
                Kind::Node(Node::Name),
                1,
            )
        },
        expect: Expect::Value("annotation"),
    },
    Case {
        name: CaseName("iter_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "for i in r: pass\n",
                Kind::Node(Node::For),
                Kind::Node(Node::Name),
                1,
            )
        },
        expect: Expect::Value("iter"),
    },
    Case {
        name: CaseName("orelse_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "for i in r: pass\nelse: pass\n",
                Kind::Node(Node::For),
                Kind::Node(Node::Pass),
                1,
            )
        },
        expect: Expect::Value("orelse"),
    },
    Case {
        name: CaseName("test_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "while t: pass\n",
                Kind::Node(Node::While),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("test"),
    },
    Case {
        name: CaseName("items_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "with o: pass\n",
                Kind::Node(Node::With),
                Kind::Node(Node::Withitem),
                0,
            )
        },
        expect: Expect::Value("items"),
    },
    Case {
        name: CaseName("context_expr_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "with o as p: pass\n",
                Kind::Node(Node::Withitem),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("context-expr"),
    },
    Case {
        name: CaseName("optional_vars_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "with o as p: pass\n",
                Kind::Node(Node::Withitem),
                Kind::Node(Node::Name),
                1,
            )
        },
        expect: Expect::Value("optional-vars"),
    },
    Case {
        name: CaseName("subject_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case 1: pass\n",
                Kind::Node(Node::Match),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("subject"),
    },
    Case {
        name: CaseName("cases_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case 1: pass\n",
                Kind::Node(Node::Match),
                Kind::Node(Node::MatchCase),
                0,
            )
        },
        expect: Expect::Value("cases"),
    },
    Case {
        name: CaseName("pattern_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case 1: pass\n",
                Kind::Node(Node::MatchCase),
                Kind::Node(Node::MatchValue),
                0,
            )
        },
        expect: Expect::Value("pattern"),
    },
    Case {
        name: CaseName("guard_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case 1 if g: pass\n",
                Kind::Node(Node::MatchCase),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("guard"),
    },
    Case {
        name: CaseName("exc_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "raise k from l\n",
                Kind::Node(Node::Raise),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("exc"),
    },
    Case {
        name: CaseName("cause_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "raise k from l\n",
                Kind::Node(Node::Raise),
                Kind::Node(Node::Name),
                1,
            )
        },
        expect: Expect::Value("cause"),
    },
    Case {
        name: CaseName("handlers_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "try: pass\nexcept E: pass\n",
                Kind::Node(Node::Try),
                Kind::Node(Node::ExceptHandler),
                0,
            )
        },
        expect: Expect::Value("handlers"),
    },
    Case {
        name: CaseName("finalbody_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "try: pass\nfinally: pass\n",
                Kind::Node(Node::Try),
                Kind::Node(Node::Pass),
                1,
            )
        },
        expect: Expect::Value("finalbody"),
    },
    Case {
        name: CaseName("msg_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "assert i, j\n",
                Kind::Node(Node::Assert),
                Kind::Node(Node::Name),
                1,
            )
        },
        expect: Expect::Value("msg"),
    },
    Case {
        name: CaseName("names_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "import a\n",
                Kind::Node(Node::Import),
                Kind::Node(Node::Alias),
                0,
            )
        },
        expect: Expect::Value("names"),
    },
    Case {
        name: CaseName("module_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "from m import a\n",
                Kind::Node(Node::ImportFrom),
                Kind::Extra(Extra::Identifier),
                0,
            )
        },
        expect: Expect::Value("module"),
    },
    Case {
        name: CaseName("level_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "from . import a\n",
                Kind::Node(Node::ImportFrom),
                Kind::Token(Token::Dot),
                0,
            )
        },
        expect: Expect::Value("level"),
    },
    Case {
        name: CaseName("values_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a and b\n",
                Kind::Node(Node::BoolOp),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("values"),
    },
    Case {
        name: CaseName("left_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a + b\n",
                Kind::Node(Node::BinOp),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("left"),
    },
    Case {
        name: CaseName("right_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a + b\n",
                Kind::Node(Node::BinOp),
                Kind::Node(Node::Name),
                1,
            )
        },
        expect: Expect::Value("right"),
    },
    Case {
        name: CaseName("operand_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || edge("-a\n", Kind::Node(Node::UnaryOp), Kind::Node(Node::Name), 0),
        expect: Expect::Value("operand"),
    },
    Case {
        name: CaseName("keys_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "{a: b}\n",
                Kind::Node(Node::Dict),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("keys"),
    },
    Case {
        name: CaseName("elt_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "[x for x in y]\n",
                Kind::Node(Node::ListComp),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("elt"),
    },
    Case {
        name: CaseName("generators_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "[x for x in y]\n",
                Kind::Node(Node::ListComp),
                Kind::Node(Node::Comprehension),
                0,
            )
        },
        expect: Expect::Value("generators"),
    },
    Case {
        name: CaseName("key_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "{k: v for k in y}\n",
                Kind::Node(Node::DictComp),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("key"),
    },
    Case {
        name: CaseName("ifs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "[x for x in y if z]\n",
                Kind::Node(Node::Comprehension),
                Kind::Node(Node::Name),
                2,
            )
        },
        expect: Expect::Value("ifs"),
    },
    Case {
        name: CaseName("is_async_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "async def f():\n    [x async for x in y]\n",
                Kind::Node(Node::Comprehension),
                Kind::Token(Token::Name),
                0,
            )
        },
        expect: Expect::Value("is-async"),
    },
    Case {
        name: CaseName("ops_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a < b\n",
                Kind::Node(Node::Compare),
                Kind::Token(Token::Less),
                0,
            )
        },
        expect: Expect::Value("ops"),
    },
    Case {
        name: CaseName("comparators_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a < b\n",
                Kind::Node(Node::Compare),
                Kind::Node(Node::Name),
                1,
            )
        },
        expect: Expect::Value("comparators"),
    },
    Case {
        name: CaseName("func_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || edge("f(x)\n", Kind::Node(Node::Call), Kind::Node(Node::Name), 0),
        expect: Expect::Value("func"),
    },
    Case {
        name: CaseName("conversion_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f'{a!r}'\n",
                Kind::Node(Node::FormattedValue),
                Kind::Token(Token::Name),
                0,
            )
        },
        expect: Expect::Value("conversion"),
    },
    Case {
        name: CaseName("format_spec_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f'{a:>3}'\n",
                Kind::Node(Node::FormattedValue),
                Kind::Node(Node::JoinedStr),
                0,
            )
        },
        expect: Expect::Value("format-spec"),
    },
    Case {
        name: CaseName("attr_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a.b\n",
                Kind::Node(Node::Attribute),
                Kind::Extra(Extra::Identifier),
                0,
            )
        },
        expect: Expect::Value("attr"),
    },
    Case {
        name: CaseName("slice_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a[1]\n",
                Kind::Node(Node::Subscript),
                Kind::Node(Node::Constant),
                0,
            )
        },
        expect: Expect::Value("slice"),
    },
    Case {
        name: CaseName("id_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || edge("a\n", Kind::Node(Node::Name), Kind::Token(Token::Name), 0),
        expect: Expect::Value("id"),
    },
    Case {
        name: CaseName("elts_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || edge("[a]\n", Kind::Node(Node::List), Kind::Node(Node::Name), 0),
        expect: Expect::Value("elts"),
    },
    Case {
        name: CaseName("lower_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a[1:2:3]\n",
                Kind::Node(Node::Slice),
                Kind::Node(Node::Constant),
                0,
            )
        },
        expect: Expect::Value("lower"),
    },
    Case {
        name: CaseName("upper_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a[1:2:3]\n",
                Kind::Node(Node::Slice),
                Kind::Node(Node::Constant),
                1,
            )
        },
        expect: Expect::Value("upper"),
    },
    Case {
        name: CaseName("step_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a[1:2:3]\n",
                Kind::Node(Node::Slice),
                Kind::Node(Node::Constant),
                2,
            )
        },
        expect: Expect::Value("step"),
    },
    Case {
        name: CaseName("patterns_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case [a]: pass\n",
                Kind::Node(Node::MatchSequence),
                Kind::Node(Node::MatchAs),
                0,
            )
        },
        expect: Expect::Value("patterns"),
    },
    Case {
        name: CaseName("rest_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case {**r}: pass\n",
                Kind::Node(Node::MatchMapping),
                Kind::Extra(Extra::Identifier),
                0,
            )
        },
        expect: Expect::Value("rest"),
    },
    Case {
        name: CaseName("cls_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case P(): pass\n",
                Kind::Node(Node::MatchClass),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("cls"),
    },
    Case {
        name: CaseName("kwd_attrs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case P(k=v): pass\n",
                Kind::Extra(Extra::PatternKeyword),
                Kind::Extra(Extra::Identifier),
                0,
            )
        },
        expect: Expect::Value("kwd-attrs"),
    },
    Case {
        name: CaseName("kwd_patterns_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "match s:\n    case P(k=v): pass\n",
                Kind::Extra(Extra::PatternKeyword),
                Kind::Node(Node::MatchAs),
                0,
            )
        },
        expect: Expect::Value("kwd-patterns"),
    },
    Case {
        name: CaseName("type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "try: pass\nexcept E: pass\n",
                Kind::Node(Node::ExceptHandler),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("type"),
    },
    Case {
        name: CaseName("posonlyargs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f(a, /): pass\n",
                Kind::Node(Node::Arguments),
                Kind::Extra(Extra::ParameterWithDefault),
                0,
            )
        },
        expect: Expect::Value("posonlyargs"),
    },
    Case {
        name: CaseName("vararg_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f(*a): pass\n",
                Kind::Node(Node::Arguments),
                Kind::Node(Node::Arg),
                0,
            )
        },
        expect: Expect::Value("vararg"),
    },
    Case {
        name: CaseName("kwonlyargs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f(*, a): pass\n",
                Kind::Node(Node::Arguments),
                Kind::Extra(Extra::ParameterWithDefault),
                0,
            )
        },
        expect: Expect::Value("kwonlyargs"),
    },
    Case {
        name: CaseName("kw_defaults_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f(*, a=1): pass\n",
                Kind::Extra(Extra::ParameterWithDefault),
                Kind::Node(Node::Constant),
                0,
            )
        },
        expect: Expect::Value("kw-defaults"),
    },
    Case {
        name: CaseName("kwarg_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f(**a): pass\n",
                Kind::Node(Node::Arguments),
                Kind::Node(Node::Arg),
                0,
            )
        },
        expect: Expect::Value("kwarg"),
    },
    Case {
        name: CaseName("defaults_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f(a=1): pass\n",
                Kind::Extra(Extra::ParameterWithDefault),
                Kind::Node(Node::Constant),
                0,
            )
        },
        expect: Expect::Value("defaults"),
    },
    Case {
        name: CaseName("arg_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f(k=1)\n",
                Kind::Node(Node::Keyword),
                Kind::Extra(Extra::Identifier),
                0,
            )
        },
        expect: Expect::Value("arg"),
    },
    Case {
        name: CaseName("bound_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f[T: int](): pass\n",
                Kind::Node(Node::TypeVar),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("bound"),
    },
    Case {
        name: CaseName("default_value_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f[T = int](): pass\n",
                Kind::Node(Node::TypeVar),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("default-value"),
    },
    Case {
        name: CaseName("asname_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "import a as b\n",
                Kind::Node(Node::Alias),
                Kind::Extra(Extra::Identifier),
                1,
            )
        },
        expect: Expect::Value("asname"),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let language = ParseContract::new(
        crate::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::build(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath("item.py"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(given.source.as_bytes()),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .map_err(drop)?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(given.source.as_bytes()),
        file: languages::FileId(0),
    };
    let found = languages::positions(&elements)
        .filter(|at| *at != Position(0))
        .filter(|at| {
            kind(&tree, parent(&tree, *at)) == given.parent && kind(&tree, *at) == given.child
        })
        .nth(given.nth)
        .ok_or(())?;
    assert_eq!(role_of(&tree, found), languages::field_at(&elements, found));
    let name = languages::text_of(
        &symbols,
        tree.source,
        languages::field_name_at(&elements, found),
    )
    .map_or("none", |name| {
        &*Box::leak(name.0.to_owned().into_boxed_str())
    });
    Ok(name)
}

/// The roles of the vocabulary no row shows.
fn roles_without_a_row() -> Vec<Field> {
    fields()
        .filter(|field| {
            let name = field_name(*field).0;
            !CASES
                .iter()
                .any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == name))
        })
        .collect()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    assert_eq!(roles_without_a_row(), Vec::new());
}
