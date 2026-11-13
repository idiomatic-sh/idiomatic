//! The cases of syntax applicability, recovery partials and required project dependencies.

use crate::application::file_analysis::errors::FileAnalysisError;
use crate::application::file_analysis::fixtures::NOTHING_HANDED;
use crate::application::file_analysis::fixtures::NOTHING_STORED;
use crate::application::file_analysis::fixtures::NOTHING_WANTED;
use crate::application::file_analysis::fixtures::prepared_test_program;
use crate::application::file_analysis::service::FileAnalysis;
use crate::application::file_analysis::types::RequestVisitSetting;
use crate::application::file_analysis::types::RuleGate;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::workers::service::tables_for;
use crate::domain::workers::types::Gating;
use ryo_execution::inbound::use_cases::metadata::interface::Mask;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_execution::inbound::use_cases::metadata::interface::RelationRead;
use ryo_execution::inbound::use_cases::metadata::interface::ScopePlacement;
use ryo_execution::outbound::matching::interface::MatchingPort;
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::runs::types::RuleId;

fn syntax_applicability_skips_only_gated_rules_and_names_the_reason()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
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
        #[cfg(feature = "fused-evaluation")]
        fused: None,
    };
    for acceptance in [SyntaxAcceptance::Rejected, SyntaxAcceptance::Undetermined] {
        seen.push(gated_acceptance(&setting, acceptance));
    }
    seen.push(accepted_and_missing_applicability(&setting));
    Ok(seen.join("|"))
}

/// What gated acceptance shows: each check in order, joined by `|`.
fn gated_acceptance(setting: &RequestVisitSetting<'_>, acceptance: SyntaxAcceptance) -> String {
    let mut seen: Vec<String> = Vec::new();
    let mut tables = tables_for(WorkerNumber(0));
    tables.mask = Mask(vec![0b11]);
    let skipped = FileAnalysis::gate_rules(
        &mut tables,
        setting,
        ryo_relations::domain::relations::types::FileId(0),
        acceptance,
    );
    seen.push(format!("{skipped:?}"));
    seen.push(format!("{:?}", tables.mask.0));
    seen.push(format!(
        "{:?}",
        FileAnalysis::active_wanted(&tables, setting, acceptance).as_ref()
    ));
    if acceptance == SyntaxAcceptance::Rejected {
        seen.push(format!("{:?}", tables.unknowns.is_empty()));
        seen.push(format!("{:?}", tables.skipped.len()));
        seen.push(format!(
            "{:?}",
            (tables.skipped[0].rule) == (setting.ids[0])
        ));
        seen.push(format!(
            "{:?}",
            (tables.skipped[0].path) == (setting.paths[0])
        ));
        seen.push(format!("{:?}", tables.skipped[0].reason));
    } else {
        seen.push(format!("{:?}", tables.unknowns.len()));
        seen.push(format!(
            "{:?}",
            (tables.unknowns[0].rule) == (setting.ids[0])
        ));
        seen.push(format!(
            "{:?}",
            (tables.unknowns[0].path) == (setting.paths[0])
        ));
        seen.push(format!("{:?}", &*tables.unknowns[0].message.0));
        seen.push(format!("{:?}", tables.skipped.is_empty()));
    }
    seen.join("|")
}

/// What accepted and missing applicability shows: each check in order, joined by `|`.
fn accepted_and_missing_applicability(setting: &RequestVisitSetting<'_>) -> String {
    let mut seen: Vec<String> = Vec::new();
    let mut tables = tables_for(WorkerNumber(0));
    tables.mask = Mask(vec![0b11]);
    seen.push(format!(
        "{:?}",
        (FileAnalysis::gate_rules(
            &mut tables,
            setting,
            ryo_relations::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        )) == ([RuleGate::Open, RuleGate::Open])
    ));
    seen.push(format!("{:?}", tables.mask.0));
    seen.push(format!(
        "{:?}",
        (FileAnalysis::active_wanted(&tables, setting, SyntaxAcceptance::Accepted).as_ref())
            == ([Relation::Function, Relation::Identifier])
    ));
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
    seen.push(format!(
        "{:?}",
        matches!(
            FileAnalysis::active_wanted(&tables, &always_setting, SyntaxAcceptance::Rejected),
            std::borrow::Cow::Borrowed(_)
        )
    ));
    seen.push(format!(
        "{:?}",
        (FileAnalysis::may_gate_rules(setting, SyntaxAcceptance::Accepted)) == (Gating::NeverGates)
    ));
    seen.push(format!(
        "{:?}",
        (FileAnalysis::may_gate_rules(setting, SyntaxAcceptance::Rejected)) == (Gating::MayGate)
    ));
    seen.push(format!("{:?}", tables.unknowns.is_empty()));

    let missing_policy = RequestVisitSetting {
        domain: crate::domain::workers::types::VisitSetting {
            applicability: &[],
            applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
            ..(setting.clone()).domain
        },
        ..setting.clone()
    };
    seen.push(format!(
        "{:?}",
        (FileAnalysis::may_gate_rules(&missing_policy, SyntaxAcceptance::Accepted))
            == (Gating::MayGate)
    ));
    let mut missing_tables = tables_for(WorkerNumber(0));
    missing_tables.mask = Mask(vec![0b11]);
    seen.push(format!(
        "{:?}",
        (FileAnalysis::gate_rules(
            &mut missing_tables,
            &missing_policy,
            ryo_relations::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        )) == ([RuleGate::Blocked, RuleGate::Blocked])
    ));
    seen.push(format!("{:?}", missing_tables.mask.0));
    seen.push(format!(
        "{:?}",
        FileAnalysis::active_wanted(&missing_tables, &missing_policy, SyntaxAcceptance::Accepted)
            .is_empty()
    ));
    seen.push(format!("{:?}", missing_tables.unknowns.len()));
    seen.push(format!("{:?}", missing_tables.skipped.is_empty()));
    seen.push(format!(
        "{:?}",
        missing_tables.unknowns.iter().all(|unknown| {
            &*unknown.message.0 == "rule skipped: applicability metadata unavailable"
        })
    ));
    seen.join("|")
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `syntax_applicability_skips_only_gated_rules_and_names_the_reason` scenario.
    SyntaxApplicabilitySkipsOnlyGatedRulesAndNamesTheReason,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = FileAnalysisError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("syntax_applicability_skips_only_gated_rules_and_names_the_reason"),
    word: Word::Succeeds,
    input: || Given::SyntaxApplicabilitySkipsOnlyGatedRulesAndNamesTheReason,
    expect: Expect::Value(
        "[Blocked, Open]|[2]|[Identifier]|true|1|true|true|SyntaxRejected|[Blocked, Open]|[2]|[Identifier]|1|true|true|\"rule skipped: syntax acceptance undetermined\"|true|true|[3]|true|true|true|true|true|true|true|[0]|true|2|true|true",
    ),
}];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::SyntaxApplicabilitySkipsOnlyGatedRulesAndNamesTheReason => {
            syntax_applicability_skips_only_gated_rules_and_names_the_reason()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
