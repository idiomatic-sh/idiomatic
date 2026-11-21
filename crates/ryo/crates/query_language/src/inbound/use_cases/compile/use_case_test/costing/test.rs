//! What a compile answers of a query's cost, from the compile inbound API over the real parser: a join on the file alone is
//! quadratic in the file unless one column is bounded on both ends or a side is proved one row a file, an order between a column
//! and itself in a join of an input with itself is quadratic in a group, a sort is linear with sorts, and everything else is
//! linear. No query here is refused: the cost is an answer.

use super::super::{Document, costed, rendered};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::costing::service::described;
use std::borrow::Cow;

/// What a row asks: the cost of a query on one line, or the lines `query validate` prints for it.
#[derive(Clone, Copy)]
enum Ask {
    /// The cost of the query, on one line.
    Cost(&'static str),
    /// The lines that word the query's cost.
    Lines(&'static str),
}

type Input = Ask;
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

/// Two relations of the file met on the file alone: every function with every comment of its file.
const PRODUCT: &str = r#"let pairs = select l.id as id, l.file as file, r.id as other from "function" as l join "comment" as r on l.file = r.file; emit pairs subject file key(id, other);"#;
/// The same pairs, kept where the comment starts inside the function's lines: one column bounded on both ends.
const RANGED: &str = r#"let spans = select id, file, "start-line" as first, "end-line" as last from "source-location"; let lines = select id, file, "start-line" as line from "source-location"; let inside = select l.id as id, l.file as file, r.id as other from lines as l join spans as r on l.file = r.file where l.line >= r.first and l.line <= r.last; emit inside subject file key(id, other);"#;
/// The same pairs with one end alone bounded: still every pair of the file.
const HALF_RANGED: &str = r#"let spans = select id, file, "start-line" as first, "end-line" as last from "source-location"; let lines = select id, file, "start-line" as line from "source-location"; let after = select l.id as id, l.file as file, r.id as other from lines as l join spans as r on l.file = r.file where l.line >= r.first; emit after subject file key(id, other);"#;
/// The same pairs, kept where the two are of one name: a further key in the `where`.
const KEYED_IN_THE_WHERE: &str = r#"let named = select id, name, file from "function"; let twins = select l.id as id, l.file as file, r.id as other from named as l join named as r on l.file = r.file where l.name = r.name; emit twins subject file key(id, other);"#;
/// Two relations met on a node and the file.
const KEYED: &str = r#"let located = select l.id as id, l.file as file, r."start-line" as line from "function" as l join "source-location" as r on l.id = r.id and l.file = r.file; emit located subject file key(id, line);"#;
/// The events of a name before each of its events, as a join of the events with themselves: every ordered pair of a name's events.
const ORDERED_PAIRS: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let earlier = select l.name as name, l.file as file, l.ordinal as point, r.ordinal as before from events as l join events as r on l.name = r.name and l.file = r.file where r.ordinal < l.ordinal; emit earlier subject file key(name, point, before);"#;
/// The same join with the earlier event bounded on both ends by the later one's row.
const ORDERED_IN_A_RANGE: &str = r#"let events = select scope, name, file, ordinal, ordinal as floor from "binding-event"; let near = select l.name as name, l.file as file, l.ordinal as point, r.ordinal as before from events as l join events as r on l.name = r.name and l.file = r.file where r.ordinal < l.ordinal and r.ordinal >= l.floor; emit near subject file key(name, point, before);"#;
/// The functions of the files that hold no comment: an antijoin on the file alone asks only whether the right side has a row.
const ANTIJOIN: &str = r#"let functions = select id, file from "function"; let commented = select file from "comment"; let bare = antijoin(left=functions, right=commented, left-on=[file], right-on=[file]); emit bare subject file key(id);"#;
/// Every function beside how many comments its file holds: the count is one row for a file.
const COUNT_FOR_A_FILE: &str = r#"let counts = group-count(from="comment", keys=["file"], count="comments"); let beside = select l.id as id, l.file as file, r.comments as comments from "function" as l join counts as r on l.file = r.file; emit beside subject file key(id, comments);"#;
/// Every function beside how many comments of each text its file holds: a row for a file and a text.
const COUNT_FOR_A_FILE_AND_A_TEXT: &str = r#"let counts = group-count(from="comment", keys=["file", "text"], count="comments"); let beside = select l.id as id, l.file as file, r.comments as comments from "function" as l join counts as r on l.file = r.file; emit beside subject file key(id, comments);"#;
/// Every function of a file that holds a comment: the files that do are one row each.
const DISTINCT_FILES: &str = r#"let commented = select file from "comment"; let kept = select l.id as id, l.file as file from "function" as l join commented as r on l.file = r.file; emit kept subject file key(id);"#;
/// Every function beside its file's parser target, which the catalog declares one row a file, read through a filter and a
/// projection of it.
const ONE_ROW_A_FILE: &str = r#"let targets = select file, minor from "language-target" where major = 3; let aimed = select l.id as id, l.file as file, r.minor as minor from "function" as l join targets as r on l.file = r.file; emit aimed subject file key(id, minor);"#;
/// Every comment beside its file's target and count, read through steps that give no more rows than they read: a join of two
/// inputs of one row a file, an antijoin of it, and its greatest row.
const THROUGH_STEPS_THAT_GIVE_NO_MORE_ROWS: &str = r#"let counts = group-count(from="comment", keys=["file"], count="comments"); let targets = select file, minor from "language-target"; let both = select l.file as file, l.comments as comments, r.minor as minor from counts as l join targets as r on l.file = r.file; let kept = antijoin(left=both, right="function", left-on=[file], right-on=[file]); let last = group-max(from=kept, keys=[file], order="comments"); let beside = select l.id as id, l.file as file, r.minor as minor from "comment" as l join last as r on l.file = r.file; emit beside subject file key(id, minor);"#;
/// Every function beside the last line of its file: the rows tied for last are all kept, so the last is not one row a file.
const LAST_OF_A_FILE: &str = r#"let lines = select id, file, "start-line" as line from "source-location"; let last = group-max(from=lines, keys=[file], order="line"); let beside = select l.id as id, l.file as file, r.line as line from "function" as l join last as r on l.file = r.file; emit beside subject file key(id, line);"#;
/// Every function beside every comment of its file, the comments read through a join with their file's count: a join with one
/// side of one row a file gives as many rows as its other side.
const COUNT_BESIDE_EVERY_COMMENT: &str = r#"let counts = group-count(from="comment", keys=["file"], count="comments"); let wide = select l.file as file, r.id as comment from counts as l join "comment" as r on l.file = r.file; let beside = select l.id as id, l.file as file, r.comment as comment from "function" as l join wide as r on l.file = r.file; emit beside subject file key(id, comment);"#;
/// One of each operation that sorts its groups and a query's text reaches here.
const SORTS: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let latest = as-of(left=events, right=events, left-on=[name, file], right-on=[name, file], left-order="ordinal", right-order="ordinal", bound=before, output={name: [left, name], file: [left, file], point: [left, ordinal], earlier: [right, ordinal]}); let first = group-min(from=events, keys=[name, file], order="ordinal"); let last = group-max(from=events, keys=[name, file], order="ordinal"); let placed = group-ordinal(from=events, keys=[name, file], order="ordinal", output="place"); let ends = union(left=first, right=last); observe latest as latest; observe placed as placed; emit ends subject file key(name, ordinal);"#;
/// A sort and a product in one query.
const SORT_AND_PRODUCT: &str = r#"let events = select scope, name, file, ordinal from "binding-event"; let last = group-max(from=events, keys=[name, file], order="ordinal"); let pairs = select l.name as name, l.file as file, r.id as other from last as l join "comment" as r on l.file = r.file; emit pairs subject file key(name, other);"#;

/// One row: the case's name, what it asks, and an answer to be read from a run.
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
        || Ask::Cost(PRODUCT),
        "QuadraticInFile [<query>::pairs Query Product QuadraticInFile]",
    ),
    row(
        "a_column_bounded_on_both_ends_is_no_product",
        || Ask::Cost(RANGED),
        "LinearWithSorts [<query>::inside Query Sort LinearWithSorts]",
    ),
    row(
        "a_column_bounded_on_one_end_is_still_a_product",
        || Ask::Cost(HALF_RANGED),
        "QuadraticInFile [<query>::after Query Product QuadraticInFile]",
    ),
    row(
        "an_equality_in_the_where_keys_a_join_on_the_file",
        || Ask::Cost(KEYED_IN_THE_WHERE),
        "Linear []",
    ),
    row(
        "a_join_keyed_beyond_the_file_is_linear",
        || Ask::Cost(KEYED),
        "Linear []",
    ),
    row(
        "an_order_of_a_column_against_itself_is_quadratic_in_a_group",
        || Ask::Cost(ORDERED_PAIRS),
        "QuadraticInGroup [<query>::earlier Query OrderedPairs QuadraticInGroup]",
    ),
    row(
        "an_order_against_itself_inside_a_range_is_a_sort",
        || Ask::Cost(ORDERED_IN_A_RANGE),
        "LinearWithSorts [<query>::near Query Sort LinearWithSorts]",
    ),
    row(
        "an_antijoin_on_the_file_alone_is_no_product",
        || Ask::Cost(ANTIJOIN),
        "Linear []",
    ),
    row(
        "a_join_with_a_count_for_each_file_is_linear",
        || Ask::Cost(COUNT_FOR_A_FILE),
        "Linear []",
    ),
    row(
        "a_count_for_a_file_and_a_text_is_not_one_row_a_file",
        || Ask::Cost(COUNT_FOR_A_FILE_AND_A_TEXT),
        "QuadraticInFile [<query>::beside Query Product QuadraticInFile]",
    ),
    row(
        "a_join_with_the_distinct_files_of_a_step_is_linear",
        || Ask::Cost(DISTINCT_FILES),
        "Linear []",
    ),
    row(
        "a_join_with_a_relation_of_one_row_a_file_is_linear",
        || Ask::Cost(ONE_ROW_A_FILE),
        "Linear []",
    ),
    row(
        "steps_that_give_no_more_rows_keep_one_row_a_file",
        || Ask::Cost(THROUGH_STEPS_THAT_GIVE_NO_MORE_ROWS),
        "LinearWithSorts [<query>::last Query Sort LinearWithSorts]",
    ),
    row(
        "the_last_of_a_file_is_not_one_row_a_file",
        || Ask::Cost(LAST_OF_A_FILE),
        "QuadraticInFile [<query>::beside Query Product QuadraticInFile; <query>::last Query Sort LinearWithSorts]",
    ),
    row(
        "a_join_with_one_side_of_one_row_a_file_is_not_one_row_a_file",
        || Ask::Cost(COUNT_BESIDE_EVERY_COMMENT),
        "QuadraticInFile [<query>::beside Query Product QuadraticInFile]",
    ),
    row(
        "each_operation_that_sorts_is_linear_with_sorts",
        || Ask::Cost(SORTS),
        "LinearWithSorts [<query>::first Query Sort LinearWithSorts; <query>::last Query Sort LinearWithSorts; <query>::latest Query Sort LinearWithSorts; <query>::placed Query Sort LinearWithSorts]",
    ),
    row(
        "the_class_is_the_greatest_of_the_statements",
        || Ask::Cost(SORT_AND_PRODUCT),
        "QuadraticInFile [<query>::last Query Sort LinearWithSorts; <query>::pairs Query Product QuadraticInFile]",
    ),
    row(
        "a_linear_query_is_worded_as_its_class",
        || Ask::Lines(KEYED),
        "linear",
    ),
    row(
        "the_first_line_names_the_statement_that_sets_the_class",
        || Ask::Lines(SORT_AND_PRODUCT),
        "quadratic in the file (statement `pairs`: a join with no key within the file) | linear with sorts (statement `last`: a sort of each group)",
    ),
    row(
        "ordered_pairs_are_worded_with_the_operations_that_sort",
        || Ask::Lines(ORDERED_PAIRS),
        "quadratic in a group (statement `earlier`: a join of an input with itself on an order within a key group, which `as-of`, `group-min`, `group-max` or `group-ordinal` says with a sort)",
    ),
];

fn subject(ask: Input) -> Result<Value, Failure> {
    let query = |text| costed(Document("<query>", text), &[]);
    Ok(Cow::Owned(match ask {
        Ask::Cost(text) => rendered(&query(text)),
        Ask::Lines(text) => described(&query(text))
            .iter()
            .map(|line| &*line.0)
            .collect::<Vec<_>>()
            .join(" | "),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
