//! The scenario `syntax_applicability_skips_only_gated_rules_and_names_the_reason` of the cases of this file.

use super::*;

pub(super) fn syntax_applicability_skips_only_gated_rules_and_names_the_reason() -> Result<Value, Failure> {
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![ryo_values::domain::plans::types::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation: ryo_values::domain::plans::types::Operation::Select {
                from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0)],
            },
        }],
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    };
    let plans = [plan.clone(), plan];
    let ids = [RuleId("gated".into()), RuleId("always".into())];
    let paths = [FilePath("source.py".into())];
    let applicability = [RuleApplicability::AcceptedSyntax, RuleApplicability::Always];
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: ryo_execution::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("empty matcher"),
    };
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, &ids, &[]);
    let setting = RequestVisitSetting {
        source_language: None,
        program,
        services: &request,
        domain: crate::domain::workers::types::VisitSetting {
            metadata: &metadata,
            wanted: &[Relation::Function, Relation::Identifier],
            paths: &paths,
            first_bucket: &[],
            ids: &ids,
            applicability: &applicability,
            applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
            placed: &placed,
            records: &[],
            reuse: &ryo_values::domain::stores::types::Reuse::Whole,
            stored: &NOTHING_STORED,
            handed: &NOTHING_HANDED,
            wanted_buckets: &NOTHING_WANTED,
            budget: ryo_values::domain::runs::types::MemoryBytes(u64::MAX),
            noted: crate::domain::visit::types::Noting::Unwanted,
            roled: crate::domain::visit::types::RoleRead::Skips,
            needs: &[
                Needs {
                    reads: vec![RelationRead {
                        relation: Relation::Function,
                        kinds: Vec::new(),
                    }],
                    ..Needs::default()
                },
                Needs {
                    reads: vec![RelationRead {
                        relation: Relation::Identifier,
                        kinds: Vec::new(),
                    }],
                    ..Needs::default()
                },
            ],
            answering: &crate::domain::render::types::Answering::Findings,
            keeping: crate::domain::visit::types::Keeping::Discards,
            unknown: &[],
        },
    };
    for (acceptance, expected_reason) in [
        (SyntaxAcceptance::Rejected, "rule skipped: syntax rejected"),
        (
            SyntaxAcceptance::Undetermined,
            "rule skipped: syntax acceptance undetermined",
        ),
    ] {
        assert_gated_acceptance(&setting, acceptance, expected_reason);
    }
    assert_accepted_and_missing_applicability(&setting);
    Ok(1)
}
