//! The `diagnostics` service: free functions named by verb over what a parser reported about one file, with no IO and no ports.

use super::constants::{
    INCOMPLETE_IMPACT, LEXICALLY_COMPLETE_IMPACT, PARSE_PHASE, PRESERVED_IMPACT, UNSPECIFIED_CODE,
    VALIDATION_PHASE,
};
use super::types::{
    Diagnostic, DiagnosticMetadata, DiagnosticPhase, Diagnostics, SyntaxCompleteness, TreeImpact,
};
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Count, Offset, ReportText};
use ryo_relations::domain::terms::types::NameText;

/// Empties what was reported, keeping its room for the next file.
pub(crate) fn clear(found: &mut Diagnostics) {
    found.text.0.clear();
    found.from.clear();
    found.to.clear();
    found.start.clear();
    found.end.clear();
    found.code.clear();
    found.phase.clear();
    found.impact.clear();
}

/// Records a diagnostic with its producer stage, stable provider code, and semantic tree impact.
pub(crate) fn record(
    found: &mut Diagnostics,
    metadata: DiagnosticMetadata,
    message: ReportText<'_>,
    start: Offset,
    end: Offset,
) {
    found.from.push(Offset(
        u32::try_from(found.text.0.len()).unwrap_or(u32::MAX),
    ));
    found.text.0.extend_from_slice(message.0.as_bytes());
    found.to.push(Offset(
        u32::try_from(found.text.0.len()).unwrap_or(u32::MAX),
    ));
    found.start.push(start);
    found.end.push(end);
    found.code.push(metadata.code);
    found.phase.push(metadata.phase);
    found.impact.push(metadata.impact);
}

/// Everything the parser reported about the file, in the order it reported it.
pub(crate) fn listed(found: &Diagnostics) -> impl Iterator<Item = Diagnostic<'_>> {
    found
        .from
        .iter()
        .zip(&found.to)
        .enumerate()
        .map(|(number, (from, to))| Diagnostic {
            message: ByteSlice(
                found
                    .text
                    .0
                    .get(
                        usize::try_from(from.0).unwrap_or(usize::MAX)
                            ..usize::try_from(to.0).unwrap_or(usize::MAX),
                    )
                    .unwrap_or(&[]),
            ),
            start: found.start.get(number).copied().unwrap_or_default(),
            end: found.end.get(number).copied().unwrap_or_default(),
            code: found.code.get(number).copied().unwrap_or(UNSPECIFIED_CODE),
            phase: found
                .phase
                .get(number)
                .copied()
                .unwrap_or(DiagnosticPhase::Parse),
            impact: found
                .impact
                .get(number)
                .copied()
                .unwrap_or(TreeImpact::Incomplete),
        })
}

/// Whether any diagnostic indicates missing or recovered syntax.
#[must_use]
pub(crate) fn incomplete_syntax(found: &Diagnostics) -> SyntaxCompleteness {
    if found.from.iter().enumerate().any(|(index, _)| {
        found
            .impact
            .get(index)
            .is_none_or(|impact| *impact != TreeImpact::Preserved)
    }) {
        SyntaxCompleteness::Incomplete
    } else {
        SyntaxCompleteness::Complete
    }
}

/// Whether lexical facts may be incomplete because tokenization failed or stopped early.
/// Missing impact metadata remains conservative: older providers do not certify lexical completeness.
#[must_use]
pub(crate) fn incomplete_lexical_stream(found: &Diagnostics) -> SyntaxCompleteness {
    if found.from.iter().enumerate().any(|(index, _)| {
        found
            .impact
            .get(index)
            .is_none_or(|impact| *impact == TreeImpact::Incomplete)
    }) {
        SyntaxCompleteness::Incomplete
    } else {
        SyntaxCompleteness::Complete
    }
}

/// How many parse and validation diagnostics the provider reported about the file.
#[must_use]
pub(crate) fn count(found: &Diagnostics) -> Count {
    Count(found.from.len())
}

/// The stable query vocabulary of a stage.
#[must_use]
pub fn phase_name(phase: DiagnosticPhase) -> NameText<'static> {
    NameText(match phase {
        DiagnosticPhase::Parse => PARSE_PHASE,
        DiagnosticPhase::Validation => VALIDATION_PHASE,
    })
}

/// The stable query vocabulary of an impact.
#[must_use]
pub fn impact_name(impact: TreeImpact) -> NameText<'static> {
    NameText(match impact {
        TreeImpact::Preserved => PRESERVED_IMPACT,
        TreeImpact::LexicallyComplete => LEXICALLY_COMPLETE_IMPACT,
        TreeImpact::Incomplete => INCOMPLETE_IMPACT,
    })
}
