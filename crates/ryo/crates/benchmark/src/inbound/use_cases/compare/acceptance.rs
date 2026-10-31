//! Comparison acceptance proves multiset semantics through the public operation.
use cucumber::then;
use ryo_benchmark::domain::diagnostics::types::{Diagnostic, RunReport};
use ryo_benchmark::inbound::use_cases::compare::service::compare;
fn reports() -> (RunReport, RunReport) {
    let first = Diagnostic {
        rule: "fixture".into(),
        file: "input.txt".into(),
        start: 0,
        end: 1,
    };
    let second = Diagnostic {
        start: 2,
        end: 3,
        ..first.clone()
    };
    let reference = RunReport {
        files: vec!["input.txt".into()],
        diagnostics: vec![first, second],
    };
    let mut candidate = reference.clone();
    candidate.diagnostics.reverse();
    (reference, candidate)
}
#[then("equivalent diagnostic multisets agree")]
fn equivalent(world: &mut crate::Behavior) {
    let _ = world;
    let (reference, candidate) = reports();
    assert_eq!(compare(&reference, &candidate), Ok(()));
}
#[then("an extra diagnostic occurrence prevents agreement")]
fn duplicate(world: &mut crate::Behavior) {
    let _ = world;
    let (reference, mut candidate) = reports();
    candidate.diagnostics.push(candidate.diagnostics[0].clone());
    let difference = compare(&reference, &candidate).expect_err("duplicate occurrence");
    assert_eq!(
        (difference.reference.len(), difference.candidate.len()),
        (2, 3)
    );
}
