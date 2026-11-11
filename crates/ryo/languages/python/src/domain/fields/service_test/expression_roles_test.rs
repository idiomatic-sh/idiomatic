//! The expression roles cases of the parent test file.

use super::edge;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

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
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
