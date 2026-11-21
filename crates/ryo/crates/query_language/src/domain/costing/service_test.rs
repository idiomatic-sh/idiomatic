//! `classified` over plans written by hand, one for each class and each part of the proof: a join on the file alone is
//! quadratic in the file, unless one column is bounded on both ends or a side is proved one row a file; an order of a column
//! against itself in a join of an input with itself is quadratic in a group; a sort is linear with sorts; a relation of pairs
//! is quadratic in a group when scanned and linear when read by a key; a statement written twice is one finding, a statement no
//! root reads is none, and the steps are read once each, whatever the number of paths that reach them.

use super::fixtures::{COMMENTS, FILE, FUNCTIONS, ROW, Written, cost, filter, join, push};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Reads, Test};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// What a row asks of the service: the cost of one plan.
#[derive(Clone, Copy)]
enum Ask {
    /// Every function with every comment of its file, the `where` given: none, a range, or one end of it.
    Pairs(Filter),
    /// The functions joined with themselves on their name and file, where one's node is before the other's.
    OrderedPairs,
    /// The same join with no order.
    KeyedPairs,
    /// Every function beside its file's row: a count of comments by file, or the parser target the catalog declares one a file.
    BesideOneRow(OneRow),
    /// The functions of the files that hold no comment.
    Antijoin,
    /// A product written twice by one statement, and one more that no root reads.
    TwiceAndUnread,
    /// A plan that scans the token links, were they a row for a pair of elements.
    PairsScanned,
    /// A plan that reads those links by the node they start from.
    PairsReadByKey,
    /// An ordered match and an ordered walk.
    OrderedOperations,
    /// A ladder of so many levels and one of twice as many: each level a union of the step before with itself, which doubles
    /// the paths to every step below, a sort of that union, and a sort no root reads.
    Ladder(usize),
}

/// The `where` of a join on the file alone.
#[derive(Clone, Copy)]
enum Filter {
    /// None.
    None,
    /// The left node at least the right one and at most the right span.
    Range,
    /// The left node at least the right one.
    HalfRange,
}

/// A side of one row a file.
#[derive(Clone, Copy)]
enum OneRow {
    /// A count of the comments by their file.
    Count,
    /// A count of the comments by their file and their text: not one row a file.
    CountByText,
    /// The parser targets.
    Declared,
}

type Input = Ask;
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

/// One row: the case's name, what it asks, and the answer a run gave.
const fn row(
    name: &'static str,
    input: fn() -> Ask,
    answer: &'static str,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(Cow::Borrowed(answer)),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    row(
        "a_join_on_the_file_alone_is_quadratic_in_the_file",
        || Ask::Pairs(Filter::None),
        "QuadraticInFile [pairs Product QuadraticInFile]",
    ),
    row(
        "a_column_bounded_on_both_ends_is_no_product",
        || Ask::Pairs(Filter::Range),
        "LinearWithSorts [pairs Sort LinearWithSorts]",
    ),
    row(
        "a_column_bounded_on_one_end_is_still_a_product",
        || Ask::Pairs(Filter::HalfRange),
        "QuadraticInFile [pairs Product QuadraticInFile]",
    ),
    row(
        "an_order_of_a_column_against_itself_is_quadratic_in_a_group",
        || Ask::OrderedPairs,
        "QuadraticInGroup [earlier OrderedPairs QuadraticInGroup]",
    ),
    row(
        "a_join_keyed_beyond_the_file_is_linear",
        || Ask::KeyedPairs,
        "Linear []",
    ),
    row(
        "a_join_with_a_count_for_each_file_is_linear",
        || Ask::BesideOneRow(OneRow::Count),
        "Linear []",
    ),
    row(
        "a_count_for_a_file_and_a_text_is_not_one_row_a_file",
        || Ask::BesideOneRow(OneRow::CountByText),
        "QuadraticInFile [beside Product QuadraticInFile]",
    ),
    row(
        "a_join_with_a_relation_of_one_row_a_file_is_linear",
        || Ask::BesideOneRow(OneRow::Declared),
        "Linear []",
    ),
    row(
        "an_antijoin_on_the_file_alone_is_no_product",
        || Ask::Antijoin,
        "Linear []",
    ),
    row(
        "a_statement_written_twice_is_one_finding_and_an_unread_one_none",
        || Ask::TwiceAndUnread,
        "QuadraticInFile [pairs Product QuadraticInFile]",
    ),
    row(
        "a_scan_of_a_relation_of_pairs_is_quadratic_in_a_group",
        || Ask::PairsScanned,
        "QuadraticInGroup [links OrderedPairs QuadraticInGroup]",
    ),
    row(
        "a_relation_of_pairs_read_by_a_key_is_linear",
        || Ask::PairsReadByKey,
        "Linear []",
    ),
    row(
        "an_ordered_match_and_an_ordered_walk_sort",
        || Ask::OrderedOperations,
        "LinearWithSorts [matched Sort LinearWithSorts; walked Sort LinearWithSorts]",
    ),
    row(
        "a_ladder_is_read_once_a_step_whatever_the_paths_to_it",
        || Ask::Ladder(16),
        "16 levels, 49 steps: 16 findings; 32 levels, 97 steps: 32 findings",
    ),
];

/// A ladder of `levels` levels over the functions, and the findings read from its last step beside how many steps it has.
fn ladder(levels: usize) -> String {
    let mut plan = Written::default();
    let sort = |from| Operation::GroupBound {
        from,
        keys: vec![ColumnIndex(1), FILE],
        order: ColumnIndex(0),
        end: GroupEnd::Greatest,
    };
    let mut last = push(&mut plan, "functions", filter(FUNCTIONS, &[]), ROW);
    for level in 0..levels {
        let union = Operation::Union {
            left: last,
            right: last,
        };
        let both = push(&mut plan, &format!("both_{level}"), union, ROW);
        let _unread = push(&mut plan, &format!("unread_{level}"), sort(both), ROW);
        last = push(&mut plan, &format!("sorted_{level}"), sort(both), ROW);
    }
    let found = cost(&plan, last);
    format!(
        "{levels} levels, {} steps: {} findings",
        plan.steps.len(),
        found.matches(" Sort ").count()
    )
}

/// The plan a row asks for, and the step its cost is read from.
fn asked(plan: &mut Written, ask: Ask) -> Reads {
    let links = Reads::Relation(Relation::TokenLink);
    match ask {
        // The join gives the left node (0), the left span (1), the right node (2) and the right span (3).
        Ask::Pairs(wanted) => {
            let pairs = push(
                plan,
                "pairs",
                join(FUNCTIONS, COMMENTS, vec![FILE], vec![FILE]),
                ROW,
            );
            let tests: &[(Test, u32, u32)] = match wanted {
                Filter::None => return pairs,
                Filter::Range => &[
                    (Test::GreaterThanOrEqual, 0, 2),
                    (Test::LessThanOrEqual, 0, 3),
                ],
                Filter::HalfRange => &[(Test::GreaterThanOrEqual, 0, 2)],
            };
            push(plan, "pairs", filter(pairs, tests), ROW)
        }
        Ask::OrderedPairs | Ask::KeyedPairs => {
            let keys = vec![ColumnIndex(1), FILE];
            let pairs = push(
                plan,
                "earlier",
                join(FUNCTIONS, FUNCTIONS, keys.clone(), keys),
                ROW,
            );
            if matches!(ask, Ask::OrderedPairs) {
                push(
                    plan,
                    "earlier",
                    filter(pairs, &[(Test::LessThan, 0, 2)]),
                    ROW,
                )
            } else {
                pairs
            }
        }
        Ask::BesideOneRow(side) => {
            let keys = match side {
                OneRow::Declared => {
                    let targets = Reads::Relation(Relation::LanguageTarget);
                    let beside = join(FUNCTIONS, targets, vec![FILE], vec![ColumnIndex(0)]);
                    return push(plan, "beside", beside, ROW);
                }
                OneRow::Count => vec![FILE],
                OneRow::CountByText => vec![FILE, ColumnIndex(1)],
            };
            let count = Operation::GroupCount {
                from: COMMENTS,
                keys,
                distinct: None,
            };
            let counts = push(plan, "counts", count, &[CellKind::File, CellKind::Number]);
            push(
                plan,
                "beside",
                join(FUNCTIONS, counts, vec![FILE], vec![ColumnIndex(0)]),
                ROW,
            )
        }
        Ask::Antijoin | Ask::PairsReadByKey => {
            let (right, on) = if matches!(ask, Ask::Antijoin) {
                (COMMENTS, FILE)
            } else {
                (links, ColumnIndex(0))
            };
            let bare = Operation::Antijoin {
                left: FUNCTIONS,
                right,
                left_on: vec![on],
                right_on: vec![on],
            };
            push(plan, "bare", bare, ROW)
        }
        Ask::TwiceAndUnread => {
            let pairs = || join(FUNCTIONS, COMMENTS, vec![FILE], vec![FILE]);
            let first = push(plan, "pairs", pairs(), ROW);
            let second = push(plan, "pairs", pairs(), ROW);
            let _unread = push(plan, "unread", pairs(), ROW);
            let both = Operation::Union {
                left: first,
                right: second,
            };
            push(plan, "both", both, ROW)
        }
        Ask::PairsScanned => push(plan, "links", filter(links, &[]), &[]),
        Ask::OrderedOperations | Ask::Ladder(_) => {
            let matched = Operation::OrderedMatch {
                left: FUNCTIONS,
                right: FUNCTIONS,
                left_on: vec![FILE],
                right_on: vec![FILE],
                left_order: ColumnIndex(0),
                right_order: ColumnIndex(0),
                output: Vec::new(),
            };
            let walked = Operation::OrderedDfs {
                from: FUNCTIONS,
                edges: Reads::Relation(Relation::SemanticChild),
                root: ColumnIndex(0),
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
                order: ColumnIndex(0),
                edge: ColumnIndex(0),
                root_keys: Vec::new(),
                edge_keys: Vec::new(),
            };
            let matched = push(plan, "matched", matched, ROW);
            let walked = push(plan, "walked", walked, ROW);
            let both = Operation::Union {
                left: matched,
                right: walked,
            };
            push(plan, "both", both, ROW)
        }
    }
}

fn subject(ask: Input) -> Result<Value, Failure> {
    if let Ask::Ladder(levels) = ask {
        let twice = ladder(levels.saturating_mul(2));
        return Ok(Cow::Owned(format!("{}; {twice}", ladder(levels))));
    }
    let mut plan = Written::default();
    let root = asked(&mut plan, ask);
    Ok(Cow::Owned(cost(&plan, root)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod packs_test;
#[cfg(test)]
mod sites_test;
