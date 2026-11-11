//! The pattern and parameter roles cases of the parent test file.

use super::edge;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;

/// What a row is given: the parent table's input.
type Input = super::Input;
/// What a row expects: the parent table's value.
type Value = super::Value;
/// What a refused row expects: the parent table's failure.
type Failure = super::Failure;

/// Runs one row through the parent table's subject.
fn subject(input: Input) -> Result<Value, Failure> {
    super::subject(input)
}

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
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

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
