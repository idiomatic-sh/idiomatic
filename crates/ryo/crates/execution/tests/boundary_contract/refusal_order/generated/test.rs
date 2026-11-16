//! Readers of refused steps, generated: every reader kind (unary, and binary in both operand orders: union, intersection,
//! difference, join, anti-join probe) over every refusal pattern (none, direct, relation, node and relation, deferred,
//! deferred of deferred, and a frontier handed or skipped), with the hosted text empty or one row, observed or not. Every
//! line the interpreter's answer at checkpoint-350 (64a99968a1) to the same generated input, recorded there in
//! `tests/fixtures/refusal_order/`.
use super::super::fixtures::programs::{
    antijoin, d, function99, h, i, join, join_functions, p, s, select_all, u, w99, words0,
};
use super::super::fixtures::{after_the_files, both_passes, per_file};
use crate::*;

/// A refusal pattern: the steps it appends after the hosted text at step 0, given the step it starts at, and the step whose
/// value is refused (or sound, for the control).
type Pattern = fn(u32) -> (Vec<Operation>, u32);

/// The patterns whose every step runs in one pass, by name.
fn patterns() -> Vec<(&'static str, Pattern)> {
    vec![
        ("none", |_| (vec![], 0)),
        ("direct", |at| (vec![p(0, &[99])], at)),
        ("relation", |at| (vec![function99()], at)),
        ("node-relation", |at| (vec![join_functions(0, 99)], at)),
        ("deferred", |at| (vec![w99(0)], at)),
        ("deferred2", |at| (vec![w99(0), w99(at)], at + 1)),
    ]
}

/// A reader of one step, by name.
type Unary = fn(u32) -> Operation;
/// A reader of two steps, by name.
type Binary = fn(u32, u32) -> Operation;

fn unary() -> Vec<(&'static str, Unary)> {
    vec![
        ("project0", |at| p(at, &[0])),
        ("project-none", |at| p(at, &[])),
        ("select-all", select_all),
        ("words0", words0),
    ]
}

fn binary() -> Vec<(&'static str, Binary)> {
    vec![
        ("union", u),
        ("intersection", i),
        ("difference", d),
        ("join", join),
        ("antijoin", antijoin),
    ]
}

/// Every generated program over one-pass patterns: its name, its operations (step 0 the hosted text) and its emitted step.
fn programs() -> Vec<(String, Vec<Operation>, u32)> {
    let mut programs = Vec::new();
    for (name, pattern) in patterns() {
        let (steps, refused) = pattern(1);
        for (reader, read) in unary() {
            let mut operations = vec![h()];
            operations.extend(steps.iter().cloned());
            operations.push(read(refused));
            let emit = u32::try_from(operations.len() - 1).expect("a few steps");
            programs.push((format!("{reader}({name})"), operations, emit));
        }
        for (reader, read) in binary() {
            for (left_first, label) in [(true, "{p},h"), (false, "h,{p}")] {
                let mut operations = vec![h()];
                operations.extend(steps.iter().cloned());
                operations.push(if left_first {
                    read(refused, 0)
                } else {
                    read(0, refused)
                });
                let emit = u32::try_from(operations.len() - 1).expect("a few steps");
                programs.push((
                    format!("{reader}({})", label.replace("{p}", name)),
                    operations,
                    emit,
                ));
            }
        }
    }
    for (left, first) in patterns() {
        for (right, second) in patterns() {
            let (mut operations, left_refused) = {
                let (steps, refused) = first(1);
                let mut operations = vec![h()];
                operations.extend(steps);
                (operations, refused)
            };
            let start = u32::try_from(operations.len()).expect("a few steps");
            let (steps, right_refused) = second(start);
            operations.extend(steps);
            for (reader, read) in binary() {
                let mut program = operations.clone();
                program.push(read(left_refused, right_refused));
                let emit = u32::try_from(program.len() - 1).expect("a few steps");
                programs.push((format!("{reader}({left},{right})"), program, emit));
            }
        }
    }
    programs
}

/// Each generated program's answers in each file (PF) and after the files with its only file skipped (XF).
fn in_each_pass() -> String {
    let mut answered = String::new();
    for (name, operations, emit) in programs() {
        for populated in [false, true] {
            for (pass, scope) in [("PF", StepScope::PerFile), ("XF", StepScope::CrossFile)] {
                let steps: Vec<_> = operations
                    .iter()
                    .cloned()
                    .map(|each| (scope, each))
                    .collect();
                for observe in [false, true] {
                    let answer = if scope == StepScope::PerFile {
                        per_file(&steps, emit, populated, observe, &[])
                    } else {
                        after_the_files(&steps, emit, vec![], populated, observe)
                    };
                    answered.push_str(&format!(
                        "{pass} {name} pop={populated} obs={observe}: {answer}\n"
                    ));
                }
            }
        }
    }
    answered
}

/// Each reader after the files of a frontier: the hosted text in the file, with a deferred refusal of it in the file or not,
/// handed over by the file (MX) or not, its only file skipped (SK).
fn across_the_files() -> String {
    let mut answered = String::new();
    let handed: Vec<(&str, Vec<Operation>, u32)> = vec![
        ("front-sound", vec![h()], 0),
        ("front-deferred", vec![h(), w99(0)], 1),
        ("front-deferred2", vec![h(), w99(0), w99(1)], 2),
    ];
    for (name, producers, refused) in handed {
        let mut readers: Vec<(String, Operation)> = unary()
            .into_iter()
            .map(|(reader, read)| (format!("{reader}({name})"), read(refused)))
            .collect();
        for (reader, read) in binary() {
            readers.push((format!("{reader}({name},h)"), read(refused, 0)));
            readers.push((format!("{reader}(h,{name})"), read(0, refused)));
        }
        for (label, reader) in readers {
            let mut steps: Vec<_> = producers
                .iter()
                .cloned()
                .map(|each| (StepScope::PerFile, each))
                .collect();
            steps.push((StepScope::CrossFile, reader));
            let plan: Vec<PlanStep> = steps
                .iter()
                .map(|(scope, operation)| PlanStep {
                    scope: *scope,
                    operation: operation.clone(),
                })
                .collect();
            let frontier =
                ryo_query_language::inbound::use_cases::compile::fixtures::rewrites::frontier(
                    &plan,
                );
            let emit = u32::try_from(steps.len() - 1).expect("a few steps");
            for populated in [false, true] {
                for observe in [false, true] {
                    let answer = both_passes(&steps, emit, frontier.clone(), populated, observe);
                    answered.push_str(&format!(
                        "MX {label} pop={populated} obs={observe}: {answer}\n"
                    ));
                    let answer =
                        after_the_files(&steps, emit, frontier.clone(), populated, observe);
                    answered.push_str(&format!(
                        "SK {label} pop={populated} obs={observe}: {answer}\n"
                    ));
                }
            }
        }
    }
    answered
}

#[test]
fn generated_readers_of_refusals_answer_as_the_interpreter_in_each_pass() {
    let answered = in_each_pass();
    assert_eq!(
        answered,
        include_str!("../../../fixtures/refusal_order/in_each_pass.txt")
    );
}

#[test]
fn generated_readers_of_a_frontier_answer_as_the_interpreter_across_the_files() {
    let answered = across_the_files();
    assert_eq!(
        answered,
        include_str!("../../../fixtures/refusal_order/across_the_files.txt")
    );
}
