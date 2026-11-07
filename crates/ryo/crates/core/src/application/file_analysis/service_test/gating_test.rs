//! The cases of syntax applicability, recovery partials and required project dependencies.
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::CorpusInventory;
use crate::outbound::corpus::types::PathStat;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_execution::inbound::use_cases::metadata::interface::RelationRead;
use ryo_execution::inbound::use_cases::metadata::interface::ScopePlacement;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution::outbound::digest::fixtures::fake::DigestFake;
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::runs::types::RuleId;
use std::sync::Arc;
use super::program_test::NOTHING_HANDED;
use super::program_test::NOTHING_STORED;
use super::program_test::NOTHING_WANTED;
use super::program_test::assert_accepted_and_missing_applicability;
use super::program_test::assert_gated_acceptance;
use super::program_test::prepared_test_program;
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::parsing::service::recovery_of;
use super::super::types::RequestVisitSetting;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::outbound::matching::interface::MatchingPort;
pub(super) fn syntax_applicability_skips_only_gated_rules_and_names_the_reason() -> Result<usize, FileAnalysisError> {
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

pub(super) fn only_incomplete_diagnostics_mark_code_relations_as_recovery() -> Result<usize, FileAnalysisError> {
    use crate::outbound::language::types::FileReading;
    use ryo_execution::inbound::use_cases::metadata::interface::Recovery;
    use ryo_language::domain::elements::service::{diagnostics, has_incomplete_lexical_stream, has_incomplete_syntax};
    use ryo_language::domain::builders::service::{diagnose, diagnose_with_metadata, table_builder};
    use ryo_language::domain::elements::types::Elements;
    use ryo_language::domain::diagnostics::types::{SyntaxCompleteness, DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, TreeImpact};
    use ryo_language::domain::symbols::types::ByteSlice;
    use ryo_relations::domain::relations::types::{Offset, ReportText};

    let source = ByteSlice(b"x");
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    {
        let mut builder = table_builder(&mut elements, &mut symbols, source);
        diagnose_with_metadata(
            &mut builder,
            DiagnosticMetadata { phase: DiagnosticPhase::Validation, impact: TreeImpact::Preserved, code: DiagnosticCode("fixture.target-syntax") },
            ReportText("syntax is unsupported by this target"),
            Offset(0),
            Offset(1),
        );
    }
    let preserved = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &elements,
    );
    assert_eq!(preserved, Recovery::Whole);
    assert_eq!(has_incomplete_syntax(&elements), SyntaxCompleteness::Complete);
    assert_eq!(
        diagnostics(&elements).count(),
        1,
        "diagnostic remains reportable"
    );
    assert!(
        MetadataUseCaseImpl
            .partial_of(&[], preserved, &[Relation::Function])
            .is_empty()
    );

    let mut lexical_elements = Elements::default();
    let mut lexical_symbols = Symbols::default();
    {
        let mut lexical_builder =
            table_builder(&mut lexical_elements, &mut lexical_symbols, source);
        diagnose_with_metadata(
            &mut lexical_builder,
            DiagnosticMetadata { phase: DiagnosticPhase::Parse, impact: TreeImpact::LexicallyComplete, code: DiagnosticCode("fixture.parser-recovered") },
            ReportText("invalid syntax, complete token stream"),
            Offset(0),
            Offset(1),
        );
    }
    let lexical_recovery = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &lexical_elements,
    );
    assert_eq!(lexical_recovery, Recovery::LexicallyCompleteSyntaxErrors);
    assert_eq!(has_incomplete_syntax(&lexical_elements), SyntaxCompleteness::Incomplete);
    assert_eq!(has_incomplete_lexical_stream(&lexical_elements), SyntaxCompleteness::Complete);
    let lexical_partials = MetadataUseCaseImpl.partial_of(
        &[],
        lexical_recovery,
        &[
            Relation::Comment,
            Relation::IndentRegion,
            Relation::Function,
        ],
    );
    assert_eq!(lexical_partials.len(), 1);
    assert_eq!(lexical_partials[0].relation, Relation::Function);
    let mut incomplete_elements = Elements::default();
    let mut incomplete_symbols = Symbols::default();
    {
        let mut validation_builder =
            table_builder(&mut incomplete_elements, &mut incomplete_symbols, source);
        diagnose(
            &mut validation_builder,
            ReportText("legacy parser diagnostic without impact metadata"),
            Offset(0),
            Offset(1),
        );
    }
    let incomplete = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &incomplete_elements,
    );
    assert_eq!(incomplete, Recovery::SyntaxErrors);
    assert_eq!(has_incomplete_syntax(&incomplete_elements), SyntaxCompleteness::Incomplete);
    assert_eq!(has_incomplete_lexical_stream(&incomplete_elements), SyntaxCompleteness::Incomplete);
    assert_eq!(diagnostics(&incomplete_elements).count(), 1);
    assert!(
        !MetadataUseCaseImpl
            .partial_of(&[], incomplete, &[Relation::Function])
            .is_empty()
    );
    Ok(1)
}

pub(super) fn required_dependency_stat_and_read_errors_cannot_disappear() -> Result<usize, FileAnalysisError> {
    for stat in [false, true] {
        let service = FileAnalysis::new(
            Arc::new(Refuses { stat }),
            Arc::new(LanguageFake::default()),
            Arc::new(DigestFake),
            Arc::new(MemoryFake::default()),
        );
        let mut result = crate::domain::project::types::Projected::default();
        assert!(matches!(
            service.consulted(&[FilePath("required".into())], &mut result),
            Err(FileAnalysisError::Corpus(_))
        ));
        assert!(result.consulted.is_empty() && result.consulted_absent.is_empty());
    }
    let service = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::new(LanguageFake::default()),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let mut result = crate::domain::project::types::Projected::default();
    service
        .consulted(&[FilePath("absent".into())], &mut result)
        .expect("a genuine missing dependency is recordable");
    assert_eq!(result.consulted_absent, vec![FilePath("absent".into())]);
    Ok(1)
}

pub(super) struct Refuses {
    stat: bool,
}

impl CorpusPort for Refuses {
    fn observe_entry(
        &self,
        path: &FilePath,
    ) -> ryo_language::domain::project_context::types::EntryObservation {
        ryo_language::domain::project_context::types::EntryObservation {
            path: path.clone(),
            state: ryo_language::domain::project_context::types::ObservedEntryState::Unknown,
        }
    }
    fn inventory(&self) -> Result<CorpusInventory, CorpusError> {
        Ok(CorpusInventory(vec![]))
    }
    fn read(&self, _: &CorpusEntry, _: &mut FileBytes) -> Result<(), CorpusError> {
        Err(CorpusError::Unreadable {
            path: "required".into(),
            reason: "read denied".into(),
        })
    }
    fn stat(&self, path: &FilePath) -> Result<PathStat, CorpusError> {
        if self.stat {
            Err(CorpusError::Unreadable {
                path: path.0.to_string(),
                reason: "stat denied".into(),
            })
        } else {
            Ok(PathStat::Found(CorpusEntry {
                executable: Default::default(),
                path: path.clone(),
                size: FileSize(1),
                modified: Nanoseconds(1),
            }))
        }
    }
}
