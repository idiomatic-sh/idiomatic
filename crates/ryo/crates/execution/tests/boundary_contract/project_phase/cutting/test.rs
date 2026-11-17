//! A join or a split made in each file, read after the files through fewer columns than it makes, is cut to them and answers the
//! rows it answered uncut, each repeated row as often; rows are compared by value, a name by its text's hash, not by the number of
//! the place its text is kept at, which a cut plan numbers anew.
use super::super::rewrites::fixtures::{
    FILE, ID, NAME, SPAN, after_the_files, column, files, functions_kept, gathered, in_each_file,
    join, rows,
};
use ryo_query_language::inbound::use_cases::compile::fixtures::rewrites::cut;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::{LiteralText, Operand, QueryLiteral, Side};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId},
    plans::types::Operation,
};

/// The rows written without the number of the place each name's text is kept at.
fn by_value(rows: Vec<String>) -> Vec<String> {
    let mut rows: Vec<String> = rows
        .into_iter()
        .map(|row| {
            row.split(", place: PlaceNumber(")
                .enumerate()
                .map(|(at, part)| {
                    if at == 0 {
                        part.to_owned()
                    } else {
                        part.split_once(')')
                            .map_or(String::new(), |(_, rest)| rest.to_owned())
                    }
                })
                .collect()
        })
        .collect();
    rows.sort_unstable();
    rows
}

#[test]
fn cutting_a_join_in_each_file_to_the_columns_read_after_the_files_keeps_the_rows() {
    let files = files();
    let kept = [ID, FILE, SPAN];
    let at = |place: u32| ColumnIndex(place);
    // The pairs of functions of each file, joined in it on the file (id, file, other id), read after the files for the two ids.
    let steps = vec![
        functions_kept(&kept),
        functions_kept(&kept),
        in_each_file(join(
            StepId(0),
            StepId(1),
            (at(1), at(1)),
            vec![
                column(Side::Left, at(0)),
                column(Side::Left, at(1)),
                column(Side::Right, at(0)),
            ],
        )),
        after_the_files(Operation::Project {
            from: Input::Step(StepId(2)),
            columns: vec![at(0), at(2)],
            distinct: RepeatedRows::Dropped,
        }),
    ];
    let emitted = StepId(3);
    let before = by_value(rows(steps.clone(), emitted, &files));
    assert!(!before.is_empty());
    let (after, after_emitted) = cut(steps.clone(), emitted);
    assert_ne!(
        after, steps,
        "the join is read after the files for two of its three columns, so it hands over those two"
    );
    assert_eq!(by_value(rows(after, after_emitted, &files)), before);
}

#[test]
fn cutting_a_join_in_each_file_keeps_the_rows_it_repeats_on_the_columns_read_after_the_files() {
    let files = files();
    let kept = [ID, NAME, FILE];
    let at = |place: u32| ColumnIndex(place);
    // The pairs of functions of each file (id, name, other id), read after the files for the id and the name, which repeat once
    // for each other function of the file, joined after the files with every name on the name.
    let steps = vec![
        functions_kept(&kept),
        functions_kept(&kept),
        in_each_file(join(
            StepId(0),
            StepId(1),
            (at(2), at(2)),
            vec![
                column(Side::Left, at(0)),
                column(Side::Left, at(1)),
                column(Side::Right, at(0)),
            ],
        )),
        functions_kept(&[NAME]),
        gathered(StepId(3), at(0)),
        after_the_files(join(
            StepId(2),
            StepId(4),
            (at(1), at(0)),
            vec![column(Side::Left, at(0))],
        )),
    ];
    let emitted = StepId(5);
    let before = by_value(rows(steps.clone(), emitted, &files));
    // Each function of the first two files is answered once for each function of its file.
    assert_eq!(before.len(), 9, "the uncut rows: {before:?}");
    let (after, after_emitted) = cut(steps.clone(), emitted);
    assert_ne!(
        after, steps,
        "the join is read after the files for two of its three columns"
    );
    assert_eq!(by_value(rows(after, after_emitted, &files)), before);
}

#[test]
fn cutting_a_split_in_each_file_keeps_the_rows_it_repeats_on_the_columns_read_after_the_files() {
    let files = files();
    let at = |place: u32| ColumnIndex(place);
    // Each function once for each word of the text "a b" appended to it (id, name, file, span, text, word), read after the files
    // for the id and the name, which repeat once for each word, joined after the files with every name on the name.
    let steps = vec![
        functions_kept(&[ID, NAME, FILE, SPAN]),
        in_each_file(Operation::Extend {
            from: Input::Step(StepId(0)),
            values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
                "a b".into(),
            )))],
        }),
        in_each_file(Operation::Words {
            from: Input::Step(StepId(1)),
            text: at(4),
        }),
        functions_kept(&[NAME]),
        gathered(StepId(3), at(0)),
        after_the_files(join(
            StepId(2),
            StepId(4),
            (at(1), at(0)),
            vec![column(Side::Left, at(0)), column(Side::Left, at(1))],
        )),
    ];
    let emitted = StepId(5);
    let before = by_value(rows(steps.clone(), emitted, &files));
    assert!(
        before.windows(2).any(|pair| pair[0] == pair[1]),
        "each function is answered once for each word: {before:?}"
    );
    let (after, after_emitted) = cut(steps.clone(), emitted);
    assert_ne!(
        after, steps,
        "the split is read after the files for two of its six columns"
    );
    assert_eq!(by_value(rows(after, after_emitted, &files)), before);
}
