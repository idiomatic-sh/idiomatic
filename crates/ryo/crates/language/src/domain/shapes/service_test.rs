//! The `shapes` service's cases, through its public functions alone: equal subtrees share a class, tokens take none, an
//! incomplete or unbuilt file has none, and the index is built once until it is cleared.

use super::service::{
    child_class, clear_shapes, prepare_shapes, push_shape_word, rewrite_shape_words, shape_class_at,
};
use super::types::{KeptShapeWords, ShapeExtent, ShapeIndex, ShapeKey, ShapeStep};
use crate::domain::builders::service::{diagnose, enter_node, leave_node, table_builder, token};
use crate::domain::elements::service::{children, end_of, flags_at, kind_at};
use crate::domain::elements::types::{Elements, TableBuilder};
use crate::domain::expansions::service::{adopt, release};
use crate::domain::keys::types::ShapeWord;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::constants::TOKEN;
use ryo_relations::domain::relations::types::{FieldId, KindBits, Offset, Position, ReportText};
use ryo_relations::domain::terms::types::Symbol;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service, observed as text.
type Input = fn() -> String;
/// What a scenario observed, each check in order, joined by `|`.
type Value = &'static str;
type Failure = ();

/// The table a scenario writes: its nodes and tokens over a short source.
fn table(write: impl FnOnce(&mut TableBuilder<'_>)) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcdefgh"));
    write(&mut builder);
    elements
}

/// A node of `kind` from `start` holding one token.
fn leaf(builder: &mut TableBuilder<'_>, kind: u16, start: u32) {
    enter_node(builder, KindBits(kind), Offset(start));
    token(builder, KindBits(9), Offset(start), Offset(start + 1));
    leave_node(builder, Offset(start + 1));
}

/// A node's key: its kind, then the class of each node child; unbuilt when a child has none.
fn kind_and_children(elements: &Elements, at: Position, key: &mut ShapeKey<'_>) -> ShapeStep {
    push_shape_word(key, ShapeWord::Kind(kind_at(elements, at)));
    for child in children(elements, at) {
        if flags_at(elements, child).0 & TOKEN.0 != 0 {
            continue;
        }
        let Some(class) = child_class(key, child) else {
            return ShapeStep::Unbuilt;
        };
        push_shape_word(key, ShapeWord::Child(FieldId(0), class));
    }
    ShapeStep::Built
}

/// Every position's class, or `None` for the whole file.
fn classes_of(elements: &Elements, classes: Option<super::types::ShapeClasses<'_>>) -> String {
    classes.map_or_else(
        || "None".to_owned(),
        |read| {
            (0..end_of(elements).0)
                .map(|at| {
                    format!(
                        "{:?}",
                        shape_class_at(&read, Position(at)).map(|class| class.0)
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        },
    )
}

/// Every position's class when each node's key is its kind and its node children's classes.
fn built_classes(elements: &Elements) -> String {
    let index = ShapeIndex::default();
    let read = prepare_shapes(&index, elements, ShapeExtent::Table, |at, key| {
        kind_and_children(elements, at, key)
    });
    classes_of(elements, read)
}

/// Two equal leaves under one root.
fn twins() -> Elements {
    table(|builder| {
        enter_node(builder, KindBits(1), Offset(0));
        leaf(builder, 2, 0);
        leaf(builder, 2, 1);
        leave_node(builder, Offset(2));
    })
}

fn equal_subtrees_share_one_class() -> String {
    built_classes(&twins())
}

fn different_words_get_different_classes() -> String {
    built_classes(&table(|builder| {
        enter_node(builder, KindBits(1), Offset(0));
        enter_node(builder, KindBits(2), Offset(0));
        leaf(builder, 5, 0);
        leave_node(builder, Offset(1));
        enter_node(builder, KindBits(2), Offset(1));
        leaf(builder, 6, 1);
        leave_node(builder, Offset(2));
        leave_node(builder, Offset(2));
    }))
}

fn tokens_are_never_offered_children_first() -> String {
    let elements = twins();
    let index = ShapeIndex::default();
    let mut offered = Vec::new();
    let read = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
        offered.push(at.0);
        kind_and_children(&elements, at, key)
    });
    format!("{}|{offered:?}", read.is_some())
}

fn incomplete_syntax_has_no_classes() -> String {
    let elements = table(|builder| {
        leaf(builder, 2, 0);
        diagnose(builder, ReportText("unclosed"), Offset(0), Offset(1));
    });
    let index = ShapeIndex::default();
    let mut calls = 0_u32;
    let read = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
        calls += 1;
        kind_and_children(&elements, at, key)
    });
    format!("{}|{calls}", classes_of(&elements, read))
}

fn an_unbuilt_node_leaves_no_classes() -> String {
    let elements = twins();
    let index = ShapeIndex::default();
    let read = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
        if at == Position(3) {
            return ShapeStep::Unbuilt;
        }
        kind_and_children(&elements, at, key)
    });
    classes_of(&elements, read)
}

fn the_walk_continues_after_unbuilt() -> String {
    let elements = twins();
    let index = ShapeIndex::default();
    let mut offered = Vec::new();
    let read = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
        offered.push(at.0);
        if at == Position(3) {
            return ShapeStep::Unbuilt;
        }
        kind_and_children(&elements, at, key)
    });
    format!("{}|{offered:?}", read.is_some())
}

fn a_skipped_node_stays_complete() -> String {
    let elements = twins();
    let index = ShapeIndex::default();
    let read = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
        if at == Position(0) {
            return ShapeStep::Skipped;
        }
        kind_and_children(&elements, at, key)
    });
    classes_of(&elements, read)
}

/// A wrapper of kind 3 around a leaf of kind 2, beside a bare leaf of kind 2.
fn wrapped() -> Elements {
    table(|builder| {
        enter_node(builder, KindBits(1), Offset(0));
        enter_node(builder, KindBits(3), Offset(0));
        leaf(builder, 2, 0);
        leave_node(builder, Offset(1));
        leaf(builder, 2, 1);
        leave_node(builder, Offset(2));
    })
}

fn same_as_takes_the_built_class() -> String {
    let elements = wrapped();
    let index = ShapeIndex::default();
    let read = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
        if at == Position(1) {
            return ShapeStep::SameAs(Position(2));
        }
        kind_and_children(&elements, at, key)
    });
    classes_of(&elements, read)
}

fn same_as_an_unclassed_node_takes_none() -> String {
    let elements = wrapped();
    let index = ShapeIndex::default();
    let read = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| match at {
        Position(1) => ShapeStep::SameAs(Position(2)),
        Position(0 | 2) => ShapeStep::Skipped,
        _ => kind_and_children(&elements, at, key),
    });
    classes_of(&elements, read)
}

fn built_once_until_cleared() -> String {
    let elements = twins();
    let mut index = ShapeIndex::default();
    let mut calls = 0_u32;
    let mut observe = |index: &ShapeIndex| {
        let read = prepare_shapes(index, &elements, ShapeExtent::Table, |at, key| {
            calls += 1;
            kind_and_children(&elements, at, key)
        });
        format!("{}:{calls}", classes_of(&elements, read))
    };
    let first = observe(&index);
    let again = observe(&index);
    clear_shapes(&mut index);
    let rebuilt = observe(&index);
    format!("{first}|{again}|{rebuilt}")
}

/// A root of two tokens, under which a node of kind 4 holding a token is added for reading.
fn adopted() -> Elements {
    table(|builder| {
        enter_node(builder, KindBits(1), Offset(0));
        token(builder, KindBits(9), Offset(0), Offset(1));
        token(builder, KindBits(9), Offset(1), Offset(2));
        leave_node(builder, Offset(2));
        adopt(builder, Position(0));
        leaf(builder, 4, 0);
        release(builder);
    })
}

fn own_tree_offers_no_added_element() -> String {
    let elements = adopted();
    let mut observed = Vec::new();
    for extent in [ShapeExtent::OwnTree, ShapeExtent::Table] {
        let index = ShapeIndex::default();
        let mut offered = Vec::new();
        let read = prepare_shapes(&index, &elements, extent, |at, key| {
            offered.push(at.0);
            push_shape_word(key, ShapeWord::Kind(kind_at(&elements, at)));
            ShapeStep::Built
        });
        observed.push(format!("{offered:?}:{}", classes_of(&elements, read)));
    }
    observed.join("|")
}

fn a_rewrite_keeps_the_compacted_prefix() -> String {
    let elements = twins();
    let index = ShapeIndex::default();
    let read = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
        push_shape_word(key, ShapeWord::Literal(Symbol(1)));
        push_shape_word(key, ShapeWord::Literal(Symbol(2)));
        if at == Position(1) {
            push_shape_word(key, ShapeWord::Literal(Symbol(2)));
            rewrite_shape_words(key, |_words| KeptShapeWords(2));
        }
        ShapeStep::Built
    });
    classes_of(&elements, read)
}

fn child_class_reads_a_built_child() -> String {
    built_classes(&table(|builder| {
        enter_node(builder, KindBits(1), Offset(0));
        enter_node(builder, KindBits(3), Offset(0));
        leaf(builder, 2, 0);
        leave_node(builder, Offset(1));
        enter_node(builder, KindBits(3), Offset(1));
        leaf(builder, 2, 1);
        leave_node(builder, Offset(2));
        leave_node(builder, Offset(2));
    }))
}

fn two_reads_borrow_at_once() -> String {
    let elements = twins();
    let index = ShapeIndex::default();
    let build = |at, key: &mut ShapeKey<'_>| kind_and_children(&elements, at, key);
    let first = prepare_shapes(&index, &elements, ShapeExtent::Table, build);
    let second = prepare_shapes(&index, &elements, ShapeExtent::Table, build);
    format!(
        "{}|{}",
        classes_of(&elements, first),
        classes_of(&elements, second)
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_subtrees_share_one_class"),
        word: Word::Succeeds,
        input: || equal_subtrees_share_one_class,
        expect: Expect::Value("Some(0),Some(3),None,Some(3),None"),
    },
    Case {
        name: CaseName("different_words_get_different_classes"),
        word: Word::Succeeds,
        input: || different_words_get_different_classes,
        expect: Expect::Value("Some(0),Some(1),Some(2),None,Some(4),Some(5),None"),
    },
    Case {
        name: CaseName("tokens_are_never_offered_children_first"),
        word: Word::Succeeds,
        input: || tokens_are_never_offered_children_first,
        expect: Expect::Value("true|[3, 1, 0]"),
    },
    Case {
        name: CaseName("incomplete_syntax_has_no_classes"),
        word: Word::Succeeds,
        input: || incomplete_syntax_has_no_classes,
        expect: Expect::Value("None|0"),
    },
    Case {
        name: CaseName("an_unbuilt_node_leaves_no_classes"),
        word: Word::Succeeds,
        input: || an_unbuilt_node_leaves_no_classes,
        expect: Expect::Value("None"),
    },
    Case {
        name: CaseName("the_walk_continues_after_unbuilt"),
        word: Word::Succeeds,
        input: || the_walk_continues_after_unbuilt,
        expect: Expect::Value("false|[3, 1, 0]"),
    },
    Case {
        name: CaseName("a_skipped_node_stays_complete"),
        word: Word::Succeeds,
        input: || a_skipped_node_stays_complete,
        expect: Expect::Value("None,Some(3),None,Some(3),None"),
    },
    Case {
        name: CaseName("same_as_takes_the_built_class"),
        word: Word::Succeeds,
        input: || same_as_takes_the_built_class,
        expect: Expect::Value("Some(0),Some(4),Some(4),None,Some(4),None"),
    },
    Case {
        name: CaseName("same_as_an_unclassed_node_takes_none"),
        word: Word::Succeeds,
        input: || same_as_an_unclassed_node_takes_none,
        expect: Expect::Value("None,None,None,None,Some(4),None"),
    },
    Case {
        name: CaseName("built_once_until_cleared"),
        word: Word::Succeeds,
        input: || built_once_until_cleared,
        expect: Expect::Value(
            "Some(0),Some(3),None,Some(3),None:3|Some(0),Some(3),None,Some(3),None:3|Some(0),Some(3),None,Some(3),None:6",
        ),
    },
    Case {
        name: CaseName("own_tree_offers_no_added_element"),
        word: Word::Succeeds,
        input: || own_tree_offers_no_added_element,
        expect: Expect::Value(
            "[0]:Some(0),None,None,None,None|[3, 0]:Some(0),None,None,Some(3),None",
        ),
    },
    Case {
        name: CaseName("a_rewrite_keeps_the_compacted_prefix"),
        word: Word::Succeeds,
        input: || a_rewrite_keeps_the_compacted_prefix,
        expect: Expect::Value("Some(3),Some(3),None,Some(3),None"),
    },
    Case {
        name: CaseName("child_class_reads_a_built_child"),
        word: Word::Succeeds,
        input: || child_class_reads_a_built_child,
        expect: Expect::Value("Some(0),Some(4),Some(5),None,Some(4),Some(5),None"),
    },
    Case {
        name: CaseName("two_reads_borrow_at_once"),
        word: Word::Succeeds,
        input: || two_reads_borrow_at_once,
        expect: Expect::Value(
            "Some(0),Some(3),None,Some(3),None|Some(0),Some(3),None,Some(3),None",
        ),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Box::leak(scenario().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
