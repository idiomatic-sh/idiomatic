//! The scenario `only_incomplete_diagnostics_mark_code_relations_as_recovery` of the cases of this file.

use super::*;

pub(super) fn only_incomplete_diagnostics_mark_code_relations_as_recovery() -> Result<Value, Failure> {
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
