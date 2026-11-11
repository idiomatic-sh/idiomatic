//! The control flow roles cases of the parent test file.

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
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
