//! The `grammar` service's cases: moving about the library's tree held as plain data, and where `go/ast` ends a node.

use super::service::{
    child_count, children, children_back, empty_label, ending, field, named, named_count,
    next_token, parent_form, spelled,
};
use super::types::{ByteIndex, Concrete, Form, GoText, Index, Naming, Presence, Raw, Role};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::GoNode;
use crate::domain::tokens::types::Token;

/// What a row asks of the small tree below.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The children of the root, in order and backwards.
    Children,
    /// How many children the root has, and how many are named and not comments.
    Counts,
    /// The child that plays a part, and the first spelling of a token.
    Parts,
    /// The form of the parent of a node.
    Parent,
    /// Where a node ends in `go/ast`.
    Ending(usize),
    /// Where the next token after a node starts.
    NextToken(usize),
    /// The place an empty label ends at.
    Label(usize),
}

type Input = Question;
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_children_of_a_node_are_met_in_order_and_back"),
        word: Word::Succeeds,
        input: || Question::Children,
        expect: Expect::Value("2 3 4 | 4 3 2"),
    },
    Case {
        name: CaseName("a_comment_is_a_child_that_is_named_but_not_among_the_named"),
        word: Word::Succeeds,
        input: || Question::Counts,
        expect: Expect::Value("3 children, 2 named children, 1 without comments"),
    },
    Case {
        name: CaseName("a_child_is_found_by_the_part_it_plays_and_a_token_by_its_spelling"),
        word: Word::Succeeds,
        input: || Question::Parts,
        expect: Expect::Value("type at 2, lbrace at 3"),
    },
    Case {
        name: CaseName("a_node_knows_the_form_of_its_parent"),
        word: Word::Succeeds,
        input: || Question::Parent,
        expect: Expect::Value("Some(Labeled)"),
    },
    Case {
        name: CaseName("a_list_of_statements_ends_where_its_last_statement_does"),
        word: Word::Succeeds,
        input: || Question::Ending(5),
        expect: Expect::Value("30"),
    },
    Case {
        name: CaseName("the_next_token_is_the_start_of_the_next_node_that_is_no_comment"),
        word: Word::Succeeds,
        input: || Question::NextToken(3),
        expect: Expect::Value("20"),
    },
    Case {
        name: CaseName("a_label_with_no_statement_ends_where_the_next_token_starts"),
        word: Word::Succeeds,
        input: || Question::Label(1),
        expect: Expect::Value("20"),
    },
];

/// The places a node of the small tree is linked to: its parent, its first and last child, and its next and previous sibling.
#[derive(Clone, Copy)]
struct Links {
    parent: Option<usize>,
    first: Option<usize>,
    last: Option<usize>,
    next: Option<usize>,
    previous: Option<usize>,
}

/// A node of the small tree.
fn raw(form: Form, role: Option<Role>, naming: Naming, span: (usize, usize), links: Links) -> Raw {
    let place = |each: Option<usize>| each.map(Index);
    Raw {
        form,
        role,
        naming,
        presence: Presence::Read,
        start: ByteIndex(span.0),
        end: ByteIndex(span.1),
        parent: place(links.parent),
        first: place(links.first),
        last: place(links.last),
        next: place(links.next),
        previous: place(links.previous),
    }
}

/// The links of a node: its parent, first child, last child, next sibling and previous sibling.
const fn at(
    parent: Option<usize>,
    first: Option<usize>,
    last: Option<usize>,
    next: Option<usize>,
    previous: Option<usize>,
) -> Links {
    Links {
        parent,
        first,
        last,
        next,
        previous,
    }
}

/// The tree of `L: // c` and a statement list: a labeled statement (1) holding a name, a typed part, a comment and a colon, and a list
/// of statements (5) holding two statements. Indexes are the places of the table, the root being 0.
fn tree() -> Concrete<'static> {
    let named = Naming::Named;
    let bare = Naming::Anonymous;
    Concrete {
        nodes: vec![
            raw(
                Form::Source,
                None,
                named,
                (0, 30),
                at(None, Some(1), Some(5), None, None),
            ),
            raw(
                Form::Labeled,
                None,
                named,
                (0, 20),
                at(Some(0), Some(2), Some(4), Some(5), None),
            ),
            raw(
                Form::Name,
                Some(Role::Type),
                named,
                (0, 1),
                at(Some(1), None, None, Some(3), None),
            ),
            raw(
                Form::Spelled(Token::Lbrace),
                None,
                bare,
                (1, 2),
                at(Some(1), None, None, Some(4), Some(2)),
            ),
            raw(
                Form::Comment,
                None,
                named,
                (3, 8),
                at(Some(1), None, None, None, Some(3)),
            ),
            raw(
                Form::StatementList,
                None,
                named,
                (20, 30),
                at(Some(0), Some(6), Some(7), None, Some(1)),
            ),
            raw(
                Form::Name,
                None,
                named,
                (20, 25),
                at(Some(5), None, None, Some(7), None),
            ),
            raw(
                Form::Node(GoNode::EmptyStmt),
                None,
                named,
                (25, 30),
                at(Some(5), None, None, None, Some(6)),
            ),
        ],
        text: GoText("L: {  // c\n                    a\n         b\n"),
    }
}

fn answer(question: Input) -> String {
    let tree = tree();
    let places = |found: Vec<Index>| {
        found
            .into_iter()
            .map(|each| each.0.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };
    match question {
        Question::Children => format!(
            "{} | {}",
            places(children(&tree, Index(1)).collect()),
            places(children_back(&tree, Index(1)).collect())
        ),
        Question::Counts => format!(
            "{} children, {} named children, {} without comments",
            child_count(&tree, Index(1)).0,
            named_count(&tree, Index(1)).0,
            named(&tree, Index(1)).count()
        ),
        Question::Parts => format!(
            "type at {}, lbrace at {}",
            field(&tree, Index(1), Role::Type).map_or(0, |each| each.0),
            spelled(&tree, Index(1), Token::Lbrace).map_or(0, |each| each.0)
        ),
        Question::Parent => format!("{:?}", parent_form(&tree, Index(2))),
        Question::Ending(at) => ending(&tree, Index(at)).0.to_string(),
        Question::NextToken(at) => next_token(&tree, Index(at)).0.to_string(),
        Question::Label(at) => empty_label(&tree, Index(at))
            .map_or_else(|| "none".to_owned(), |each| each.0.to_string()),
    }
}

fn subject(question: Input) -> Value {
    Box::leak(answer(question).into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
