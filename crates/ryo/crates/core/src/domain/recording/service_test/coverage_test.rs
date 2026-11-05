//! Rejected syntax preserves coverage for independently applicable rules.
use super::*;
fn rejected_syntax_coverage_ignores_partials_for_gated_rules_after_mask_restore() {
    use crate::domain::workers::types::VisitSetting;
    use ryo_execution::inbound::use_cases::metadata::interface::Mask;
    use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
    use ryo_language::domain::elements::types::SyntaxAcceptance;
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::runs::types::{RuleApplicability, RuleId};

    let placed = placed().expect("empty placement is valid");
    let plans = [plan_with_a_frontier(), plan_with_a_frontier()];
    let ids = [RuleId("syntax-gated".into()), RuleId("always".into())];
    let applicability = [RuleApplicability::AcceptedSyntax, RuleApplicability::Always];
    let needs = [
        Needs {
            reads: vec![RelationRead {
                relation: Relation::Defines,
                kinds: Vec::new(),
            }],
            ..Needs::default()
        },
        Needs {
            reads: vec![RelationRead {
                relation: Relation::Defines,
                kinds: Vec::new(),
            }],
            ..Needs::default()
        },
    ];
    let path = FilePath("invalid.py".into());
    let paths = [path.clone()];
    let setting = VisitSetting {
        metadata: &from_plans(&plans),
        ids: &ids,
        applicability: &applicability,
        applicability_can_gate: true,
        needs: &needs,
        paths: &paths,
        ..setting(&placed)
    };
    let mut worker = tables();
    worker.mask = Mask(vec![0b11]);
    worker.partials = vec![partial()];

    // `read_file` restores the original cross-rule mask before `cover` runs.
    worker.mask = Mask(vec![0b11]);
    cover(
        &mut worker,
        &setting,
        FileId(0),
        &path,
        SyntaxAcceptance::Rejected,
    );

    assert_eq!(worker.unknowns.len(), 1);
    assert_eq!(worker.unknowns[0].rule, ids[1]);
    assert_eq!(worker.unknowns[0].path, path);
    assert!(worker.covered.is_empty());
}

type Input = ();
type Value = ();
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("rejected_syntax_preserves_always_applicable_coverage"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: ()) -> Result<(), ()> {
    rejected_syntax_coverage_ignores_partials_for_gated_rules_after_mask_restore();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
