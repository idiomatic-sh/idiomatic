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

/// The unknowns of one file visit: the engine's partial root of the one rule, its `affected` partitions, then `cover`.
fn unknowns_after(affected: &[Option<FileId>]) -> Vec<(String, String)> {
    use crate::domain::recording::service::partial_root;
    use ryo_execution::inbound::use_cases::metadata::interface::Mask;
    use ryo_values::domain::runs::types::RuleId;

    let placed = placed().expect("empty placement is valid");
    let needs = [needs()];
    let ids = [RuleId("rule".into())];
    let setting = VisitSetting {
        needs: &needs,
        ids: &ids,
        ..setting(&placed)
    };
    let path = FilePath("a.rs".into());
    let mut worker = tables();
    worker.partials = vec![partial()];
    worker.mask = Mask(vec![0b1]);
    partial_root(
        &mut worker.unknowns,
        &worker.partials,
        &setting,
        RuleIndex(0),
        FileId(3),
        &path,
        affected,
    );
    cover(
        &mut worker,
        &setting,
        FileId(3),
        &path,
        ryo_language::domain::elements::types::SyntaxAcceptance::Accepted,
    );
    worker
        .unknowns
        .iter()
        .map(|unknown| (unknown.rule.0.to_string(), unknown.path.0.to_string()))
        .collect()
}

/// A root partial only through its own file's partial relation is one unknown, the one `cover` gives with its reason; a root
/// partial through another file or the repository is also the rule's own unknown, which `cover` cannot see.
fn own_partiality_is_one_unknown_and_foreign_partiality_is_its_own() {
    let one = vec![("rule".to_owned(), "a.rs".to_owned())];
    assert_eq!(unknowns_after(&[Some(FileId(3))]), one);
    let two = vec![
        ("rule".to_owned(), "a.rs".to_owned()),
        ("rule".to_owned(), "a.rs".to_owned()),
    ];
    assert_eq!(unknowns_after(&[None]), two);
    assert_eq!(unknowns_after(&[Some(FileId(3)), Some(FileId(4))]), two);
    // The project phase asks the same of each affected file: named there only when the rule reads the relation read in part.
    use crate::domain::recording::service::names_at_file;
    use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
    use ryo_relations::domain::relations::types::Relation;
    assert!(names_at_file(&[partial()], Some(&needs())));
    let elsewhere = Needs {
        reads: vec![RelationRead {
            relation: Relation::Function,
            kinds: Vec::new(),
        }],
        ..Needs::default()
    };
    assert!(!names_at_file(&[partial()], Some(&elsewhere)));
    assert!(!names_at_file(&[partial()], None));
}

type Input = fn();
type Value = ();
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rejected_syntax_preserves_always_applicable_coverage"),
        word: Word::Succeeds,
        input: || rejected_syntax_coverage_ignores_partials_for_gated_rules_after_mask_restore,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("own_partiality_is_one_unknown_and_foreign_partiality_is_its_own"),
        word: Word::Succeeds,
        input: || own_partiality_is_one_unknown_and_foreign_partiality_is_its_own,
        expect: Expect::Value(()),
    },
];
fn subject(scenario: fn()) -> Result<(), ()> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
