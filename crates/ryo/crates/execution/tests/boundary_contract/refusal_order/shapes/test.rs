//! Reviewed shapes in the refused-reader class, each line the interpreter's answer at checkpoint-350 (64a99968a1) to the same
//! input, recorded there in `tests/fixtures/refusal_order/`.
use super::super::fixtures::programs::{bad, d, h, join_functions, p, s, u, w99};
use super::super::fixtures::{after_the_files, both_passes, per_file};
use crate::*;
use ryo_values::domain::plan_parts::types::{JoinColumn, Side};

const PF: StepScope = StepScope::PerFile;
const XF: StepScope = StepScope::CrossFile;

/// A join of the function relation to step `at`, the function's node against the step's column `key`.
fn join_rel_left(at: u32, key: u32) -> Operation {
    Operation::Join {
        left: Input::Relation(Relation::Function),
        right: s(at),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(key)],
        output: vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(0),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    }
}

/// Every reviewed shape's answers, in the order the review listed them.
fn answered() -> String {
    let mut out = String::new();
    let mixed: Vec<(&str, Vec<(StepScope, Operation)>, u32, Vec<Input>)> = vec![
        (
            "mx-union-front-refused",
            vec![(PF, h()), (PF, w99(0)), (XF, p(0, &[99])), (XF, u(1, 2))],
            3,
            vec![s(0), s(1)],
        ),
        (
            "mx-union-refused-front",
            vec![(PF, h()), (PF, w99(0)), (XF, p(0, &[99])), (XF, u(2, 1))],
            3,
            vec![s(0), s(1)],
        ),
        (
            "mx-diff-front-refused",
            vec![(PF, h()), (PF, w99(0)), (XF, p(0, &[99])), (XF, d(1, 2))],
            3,
            vec![s(0), s(1)],
        ),
        (
            "mx-diff-refused-front",
            vec![(PF, h()), (PF, w99(0)), (XF, p(0, &[99])), (XF, d(2, 1))],
            3,
            vec![s(0), s(1)],
        ),
        (
            "mx-bad-front-w99",
            vec![(PF, h()), (PF, bad(0)), (XF, w99(1)), (XF, p(2, &[0]))],
            3,
            vec![s(1)],
        ),
        (
            "mx-xf-nested",
            vec![(PF, h()), (XF, w99(0)), (XF, p(1, &[0])), (XF, p(2, &[0]))],
            3,
            vec![s(0)],
        ),
        (
            "mx-xf-unread",
            vec![(PF, h()), (XF, w99(0)), (XF, p(1, &[0])), (XF, p(1, &[1]))],
            3,
            vec![s(0)],
        ),
        (
            "mx-front-w99-of-w99",
            vec![(PF, h()), (PF, w99(0)), (XF, w99(1))],
            2,
            vec![s(1)],
        ),
        (
            "mx-xf-join-front",
            vec![(PF, h()), (PF, w99(0)), (XF, join_functions(1, 0))],
            2,
            vec![s(1)],
        ),
    ];
    for (name, steps, emit, frontier) in &mixed {
        for populated in [false, true] {
            for observe in [false, true] {
                let a = both_passes(steps, *emit, frontier.clone(), populated, observe);
                out.push_str(&format!("MX {name} pop={populated} obs={observe}: {a}\n"));
            }
        }
    }
    let skipped: Vec<(&str, Vec<(StepScope, Operation)>, u32, Vec<Input>)> = vec![
        (
            "sk-union-front-refused",
            vec![(PF, h()), (PF, w99(0)), (XF, p(0, &[99])), (XF, u(1, 2))],
            3,
            vec![s(0), s(1)],
        ),
        (
            "sk-union-refused-front",
            vec![(PF, h()), (PF, w99(0)), (XF, p(0, &[99])), (XF, u(2, 1))],
            3,
            vec![s(0), s(1)],
        ),
    ];
    for (name, steps, emit, frontier) in &skipped {
        for populated in [false, true] {
            for observe in [false, true] {
                let a = after_the_files(steps, *emit, frontier.clone(), populated, observe);
                out.push_str(&format!("SK {name} pop={populated} obs={observe}: {a}\n"));
            }
        }
    }
    let table = source();
    let nr = [
        ("rel-left-w99", vec![h(), w99(0), join_rel_left(1, 0)], 2),
        ("rel-left-node99", vec![h(), join_rel_left(0, 99)], 1),
        (
            "rel-join-under-union",
            vec![h(), w99(0), join_functions(1, 0), u(2, 0)],
            3,
        ),
        ("rel-join-bad", vec![h(), bad(0), join_functions(1, 99)], 2),
        (
            "rel-join-unread",
            vec![h(), w99(0), join_functions(1, 0), p(0, &[0])],
            3,
        ),
    ];
    for (name, ops, emit) in &nr {
        for (scope, pass) in [(PF, "PF"), (XF, "XF")] {
            let steps: Vec<_> = ops.iter().cloned().map(|o| (scope, o)).collect();
            for (rel, facts) in [
                ("refused", vec![]),
                ("read", vec![(Relation::Function, &table)]),
            ] {
                for populated in [false, true] {
                    for observe in [false, true] {
                        let a = if scope == PF {
                            per_file(&steps, *emit, populated, observe, &facts)
                        } else if rel == "read" {
                            continue;
                        } else {
                            after_the_files(&steps, *emit, vec![], populated, observe)
                        };
                        out.push_str(&format!(
                            "NR {pass} {name} function={rel} pop={populated} obs={observe}: {a}\n"
                        ));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn reviewed_refused_reader_shapes_answer_as_the_interpreter() {
    assert_eq!(
        answered(),
        include_str!("../../../fixtures/refusal_order/reviewed_shapes.txt")
    );
}
