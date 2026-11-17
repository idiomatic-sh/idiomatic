//! A step reading a row-by-row refusal reads no column of it: a reader of several columns is refused at the first of them,
//! once the refusing step ran without a refusal of its own, whatever a kernel over the refusing step's layout would find.
use crate::mode_order::fixtures::*;
use crate::*;

/// The multi-column readers of step 1, by name.
fn readers() -> Vec<(&'static str, Operation)> {
    vec![
        ("project-0-99", project(&[0, 99])),
        (
            "select-0-99",
            Operation::Select {
                from: Input::Step(StepId(1)),
                comparisons: vec![],
                kept: vec![ColumnIndex(0), ColumnIndex(99)],
            },
        ),
    ]
}

/// What the interpreter at checkpoint-350 (64a99968a1) answered for each case, run there and recorded.
const MULTI_COLUMN_READERS: &str = concat!(
    "capture-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "capture-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "capture-col99 populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "capture-col99 populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "capture-num populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "capture-num populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "capture-num populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "capture-num populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "rank-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "rank-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "rank-col99 populated=true reader=project-0-99: failed Unbound\n",
    "rank-col99 populated=true reader=select-0-99: failed Unbound\n",
    "words-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "words-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "words-col99 populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "words-col99 populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "words-num populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "words-num populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "words-num populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "words-num populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "replace-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "replace-col99 populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-col99 populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-num populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-num populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-num populated=true reader=project-0-99: failed ShapeMismatch\n",
    "replace-num populated=true reader=select-0-99: failed ShapeMismatch\n",
    "replace-badre populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-badre populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-badre populated=true reader=project-0-99: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "replace-badre populated=true reader=select-0-99: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "spanbetween-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbetween-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbetween-col99 populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbetween-col99 populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbetween-text populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbetween-text populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbetween-text populated=true reader=project-0-99: failed ShapeMismatch\n",
    "spanbetween-text populated=true reader=select-0-99: failed ShapeMismatch\n",
    "sourcematches-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "sourcematches-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "sourcematches-col99 populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "sourcematches-col99 populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "sourcematches-text populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "sourcematches-text populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "sourcematches-text populated=true reader=project-0-99: failed ShapeMismatch\n",
    "sourcematches-text populated=true reader=select-0-99: failed ShapeMismatch\n",
    "render-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "render-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "render-col99 populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "render-col99 populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "render-text populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "render-text populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "render-text populated=true reader=project-0-99: failed ShapeMismatch\n",
    "render-text populated=true reader=select-0-99: failed ShapeMismatch\n",
    "fits-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "fits-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "fits-col99 populated=true reader=project-0-99: failed ShapeMismatch\n",
    "fits-col99 populated=true reader=select-0-99: failed ShapeMismatch\n",
    "fits-spantext populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "fits-spantext populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "fits-spantext populated=true reader=project-0-99: failed ShapeMismatch\n",
    "fits-spantext populated=true reader=select-0-99: failed ShapeMismatch\n",
    "spanbounds-col99 populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbounds-col99 populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbounds-col99 populated=true reader=project-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbounds-col99 populated=true reader=select-0-99: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbounds-text populated=false reader=project-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbounds-text populated=false reader=select-0-99: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbounds-text populated=true reader=project-0-99: failed ShapeMismatch\n",
    "spanbounds-text populated=true reader=select-0-99: failed ShapeMismatch\n",
);

#[test]
fn multi_column_readers_of_a_row_by_row_refusal_answer_as_the_interpreter() {
    let mut answered = String::new();
    for (name, operation) in row_by_row_steps() {
        for populated in [false, true] {
            for (reader, read) in readers() {
                let outcome = row_by_row(operation.clone(), populated, Some(read));
                answered.push_str(&format!(
                    "{name} populated={populated} reader={reader}: {outcome}\n"
                ));
            }
        }
    }
    assert_eq!(answered, MULTI_COLUMN_READERS);
}
