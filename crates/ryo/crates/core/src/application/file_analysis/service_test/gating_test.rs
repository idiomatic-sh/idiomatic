//! The gating of the cases of this file.

use super::*;

/// Check a rejected or undetermined source without mixing its owner tables with other cases.
pub(super) fn assert_gated_acceptance(
    setting: &RequestVisitSetting<'_>,
    acceptance: SyntaxAcceptance,
    expected_reason: &str,
) {
    let mut tables = tables_for(WorkerNumber(0), setting.metadata);
    tables.mask = Mask(vec![0b11]);
    let skipped = FileAnalysis::gate_rules(
        &mut tables,
        setting,
        ryo_relations::domain::relations::types::FileId(0),
        acceptance,
    );
    assert_eq!(skipped, [RuleGate::Blocked, RuleGate::Open]);
    assert_eq!(tables.mask.0, [0b10]);
    assert_eq!(
        FileAnalysis::active_wanted(&tables, setting, acceptance).as_ref(),
        [Relation::Identifier]
    );
    if acceptance == SyntaxAcceptance::Rejected {
        assert!(tables.unknowns.is_empty());
        assert_eq!(tables.skipped.len(), 1);
        assert_eq!(tables.skipped[0].rule, setting.ids[0]);
        assert_eq!(tables.skipped[0].path, setting.paths[0]);
        assert_eq!(
            tables.skipped[0].reason,
            ryo_values::domain::runs::types::SkipReason::SyntaxRejected
        );
        assert!(expected_reason.contains("rejected"));
    } else {
        assert_eq!(tables.unknowns.len(), 1);
        assert_eq!(tables.unknowns[0].rule, setting.ids[0]);
        assert_eq!(tables.unknowns[0].path, setting.paths[0]);
        assert_eq!(&*tables.unknowns[0].message.0, expected_reason);
        assert!(tables.skipped.is_empty());
    }
}

/// Accepted syntax keeps enabled rules; absent applicability metadata still records unknowns.
pub(super) fn assert_accepted_and_missing_applicability(setting: &RequestVisitSetting<'_>) {
    let mut tables = tables_for(WorkerNumber(0), setting.metadata);
    tables.mask = Mask(vec![0b11]);
    assert_eq!(
        FileAnalysis::gate_rules(
            &mut tables,
            setting,
            ryo_relations::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        ),
        [RuleGate::Open, RuleGate::Open]
    );
    assert_eq!(tables.mask.0, [0b11]);
    assert_eq!(
        FileAnalysis::active_wanted(&tables, setting, SyntaxAcceptance::Accepted).as_ref(),
        [Relation::Function, Relation::Identifier]
    );
    let always_setting = RequestVisitSetting {
        source_language: None,
        domain: crate::domain::workers::types::VisitSetting {
            metadata: &setting.metadata[1..],
            ids: &setting.ids[1..],
            applicability: &setting.applicability[1..],
            applicability_can_gate: crate::domain::workers::types::Gating::NeverGates,
            needs: &setting.needs[1..],
            ..(setting.clone()).domain
        },
        ..setting.clone()
    };
    assert!(matches!(
        FileAnalysis::active_wanted(&tables, &always_setting, SyntaxAcceptance::Rejected),
        std::borrow::Cow::Borrowed(_)
    ));
    assert_eq!(
        FileAnalysis::may_gate_rules(setting, SyntaxAcceptance::Accepted),
        Gating::NeverGates
    );
    assert_eq!(
        FileAnalysis::may_gate_rules(setting, SyntaxAcceptance::Rejected),
        Gating::MayGate
    );
    assert!(tables.unknowns.is_empty());

    let missing_policy = RequestVisitSetting {
        domain: crate::domain::workers::types::VisitSetting {
            applicability: &[],
            applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
            ..(setting.clone()).domain
        },
        ..setting.clone()
    };
    assert_eq!(
        FileAnalysis::may_gate_rules(&missing_policy, SyntaxAcceptance::Accepted),
        Gating::MayGate
    );
    let mut missing_tables = tables_for(WorkerNumber(0), setting.metadata);
    missing_tables.mask = Mask(vec![0b11]);
    assert_eq!(
        FileAnalysis::gate_rules(
            &mut missing_tables,
            &missing_policy,
            ryo_relations::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        ),
        [RuleGate::Blocked, RuleGate::Blocked]
    );
    assert_eq!(missing_tables.mask.0, [0]);
    assert!(
        FileAnalysis::active_wanted(&missing_tables, &missing_policy, SyntaxAcceptance::Accepted)
            .is_empty()
    );
    assert_eq!(missing_tables.unknowns.len(), 2);
    assert!(missing_tables.skipped.is_empty());
    assert!(missing_tables.unknowns.iter().all(|unknown| {
        &*unknown.message.0 == "rule skipped: applicability metadata unavailable"
    }));
}
