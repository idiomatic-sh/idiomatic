//! The operations the plan that runs lowers to a typed step are all operations the checks know, by name: a query the executor runs is
//! never refused by the checks as having no such operation.

use crate::domain::syntax::constants::operator_named;

/// The names of the `OP_*` constants of the plan's vocabulary, read from the file that states them.
fn lowered_names() -> Vec<String> {
    let constants = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../values/src/domain/finish_plans/constants.rs");
    let text = std::fs::read_to_string(constants).expect("the plan's constants");
    text.lines()
        .filter(|line| line.starts_with("pub const OP_"))
        .filter_map(|line| line.split('"').nth(1).map(str::to_owned))
        .collect()
}

#[test]
fn every_operation_the_plan_that_runs_lowers_is_known_to_the_checks_by_name() {
    let names = lowered_names();
    assert!(names.len() >= 25, "the plan's operation names were not read: {names:?}");
    let unknown: Vec<_> = names.iter().filter(|name| operator_named(name).is_none()).collect();
    assert!(unknown.is_empty(), "operations the plan lowers and the checks do not know: {unknown:?}");
}

/// The operations the executor runs, read from its dispatch (the arms of `Exec::step` that name an operation), but for the base
/// relation, which no query calls.
fn run_names() -> Vec<String> {
    let executor = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../executor/src/finish.rs");
    let text = std::fs::read_to_string(executor).expect("the executor's dispatch");
    let start = text.find("pub fn step(&mut self, f: &mut File").expect("the dispatch");
    let body = &text[start..];
    let body = &body[body.find("match op {").expect("the match")..];
    let mut names = Vec::new();
    for line in body.lines().skip(1) {
        if line.starts_with("            _ =>") {
            break;
        }
        if line.starts_with("            \"") && line.trim_end().ends_with("=> {") || line.starts_with("            \"") && line.contains(" => self.") {
            let arms = line.split("=>").next().unwrap_or("");
            names.extend(arms.split('"').skip(1).step_by(2).map(str::to_owned));
        }
    }
    names.retain(|name| name != "base");
    names
}

#[test]
fn every_operation_the_executor_runs_is_known_to_the_checks_by_name() {
    let names = run_names();
    assert!(names.len() >= 20, "the executor's operation names were not read: {names:?}");
    let unknown: Vec<_> = names.iter().filter(|name| operator_named(name).is_none()).collect();
    assert!(unknown.is_empty(), "operations the executor runs and the checks do not know: {unknown:?}");
}
