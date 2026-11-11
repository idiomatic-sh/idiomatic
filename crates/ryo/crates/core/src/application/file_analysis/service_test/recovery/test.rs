//! Recovery marks only the code relations of incomplete diagnostics, and a required dependency's stat and read errors are kept.

use crate::application::file_analysis::errors::FileAnalysisError;
use crate::application::file_analysis::fixtures::gating::Refuses;
use crate::application::file_analysis::parsing::service::recovery_of;
use crate::application::file_analysis::service::FileAnalysis;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

fn only_incomplete_diagnostics_mark_code_relations_as_recovery() -> Result<String, FileAnalysisError>
{
    let mut seen: Vec<String> = Vec::new();
    use crate::outbound::language::types::FileReading;

    use ryo_language::domain::builders::service::{
        diagnose, diagnose_with_metadata, table_builder,
    };
    use ryo_language::domain::diagnostics::types::{
        DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, TreeImpact,
    };
    use ryo_language::domain::elements::service::{
        diagnostics, has_incomplete_lexical_stream, has_incomplete_syntax,
    };
    use ryo_language::domain::elements::types::Elements;
    use ryo_language::domain::symbols::types::ByteSlice;
    use ryo_relations::domain::relations::types::{Offset, ReportText};

    let source = ByteSlice(b"x");
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    {
        let mut builder = table_builder(&mut elements, &mut symbols, source);
        diagnose_with_metadata(
            &mut builder,
            DiagnosticMetadata {
                phase: DiagnosticPhase::Validation,
                impact: TreeImpact::Preserved,
                code: DiagnosticCode("fixture.target-syntax"),
            },
            ReportText("syntax is unsupported by this target"),
            Offset(0),
            Offset(1),
        );
    }
    let preserved = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &elements,
    );
    seen.push(format!("{:?}", preserved));
    seen.push(format!("{:?}", has_incomplete_syntax(&elements)));
    seen.push(format!("{:?}", diagnostics(&elements).count()));
    seen.push(format!(
        "{:?}",
        MetadataUseCaseImpl
            .partial_of(&[], preserved, &[Relation::Function])
            .is_empty()
    ));

    let mut lexical_elements = Elements::default();
    let mut lexical_symbols = Symbols::default();
    {
        let mut lexical_builder =
            table_builder(&mut lexical_elements, &mut lexical_symbols, source);
        diagnose_with_metadata(
            &mut lexical_builder,
            DiagnosticMetadata {
                phase: DiagnosticPhase::Parse,
                impact: TreeImpact::LexicallyComplete,
                code: DiagnosticCode("fixture.parser-recovered"),
            },
            ReportText("invalid syntax, complete token stream"),
            Offset(0),
            Offset(1),
        );
    }
    let lexical_recovery = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &lexical_elements,
    );
    seen.push(format!("{:?}", lexical_recovery));
    seen.push(format!("{:?}", has_incomplete_syntax(&lexical_elements)));
    seen.push(format!(
        "{:?}",
        has_incomplete_lexical_stream(&lexical_elements)
    ));
    let lexical_partials = MetadataUseCaseImpl.partial_of(
        &[],
        lexical_recovery,
        &[
            Relation::Comment,
            Relation::IndentRegion,
            Relation::Function,
        ],
    );
    seen.push(format!("{:?}", lexical_partials.len()));
    seen.push(format!("{:?}", lexical_partials[0].relation));
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
    seen.push(format!("{:?}", incomplete));
    seen.push(format!("{:?}", has_incomplete_syntax(&incomplete_elements)));
    seen.push(format!(
        "{:?}",
        has_incomplete_lexical_stream(&incomplete_elements)
    ));
    seen.push(format!("{:?}", diagnostics(&incomplete_elements).count()));
    seen.push(format!(
        "{:?}",
        !MetadataUseCaseImpl
            .partial_of(&[], incomplete, &[Relation::Function])
            .is_empty()
    ));
    Ok(seen.join("|"))
}

fn required_dependency_stat_and_read_errors_cannot_disappear() -> Result<String, FileAnalysisError>
{
    let mut seen: Vec<String> = Vec::new();
    for stat in [false, true] {
        let service = FileAnalysis::new(
            Arc::new(Refuses { stat }),
            Arc::new(LanguageFake::default()),
            Arc::new(DigestFake),
            Arc::new(MemoryFake::default()),
        );
        let mut result = crate::domain::project::types::Projected::default();
        seen.push(format!(
            "{:?}",
            matches!(
                service.consulted(&[FilePath("required".into())], &mut result),
                Err(FileAnalysisError::Corpus(_))
            )
        ));
        seen.push(format!(
            "{:?}",
            result.consulted.is_empty() && result.consulted_absent.is_empty()
        ));
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
    seen.push(format!(
        "{:?}",
        (result.consulted_absent) == (vec![FilePath("absent".into())])
    ));
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `only_incomplete_diagnostics_mark_code_relations_as_recovery` scenario.
    OnlyIncompleteDiagnosticsMarkCodeRelationsAsRecovery,
    /// The `required_dependency_stat_and_read_errors_cannot_disappear` scenario.
    RequiredDependencyStatAndReadErrorsCannotDisappear,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = FileAnalysisError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("only_incomplete_diagnostics_mark_code_relations_as_recovery"),
        word: Word::Succeeds,
        input: || Given::OnlyIncompleteDiagnosticsMarkCodeRelationsAsRecovery,
        expect: Expect::Value(
            "Whole|Complete|1|true|LexicallyCompleteSyntaxErrors|Incomplete|Complete|1|Function|SyntaxErrors|Incomplete|Incomplete|1|true",
        ),
    },
    Case {
        name: CaseName("required_dependency_stat_and_read_errors_cannot_disappear"),
        word: Word::Succeeds,
        input: || Given::RequiredDependencyStatAndReadErrorsCannotDisappear,
        expect: Expect::Value("true|true|true|true|true"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::OnlyIncompleteDiagnosticsMarkCodeRelationsAsRecovery => {
            only_incomplete_diagnostics_mark_code_relations_as_recovery()?
        }
        Given::RequiredDependencyStatAndReadErrorsCannotDisappear => {
            required_dependency_stat_and_read_errors_cannot_disappear()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
