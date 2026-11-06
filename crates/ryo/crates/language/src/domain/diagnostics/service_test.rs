//! The `diagnostics` service's cases, on values alone.

use super::constants::UNSPECIFIED_CODE;
use super::service::{
    clear, count, impact_name, incomplete_lexical_stream, incomplete_syntax, listed, phase_name,
    record,
};
use super::types::{
    DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, Diagnostics, SyntaxCompleteness,
    TreeImpact,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{Offset, ReportText};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// A report of the given impact, recorded with a code of the fixture.
fn reported(found: &mut Diagnostics, impact: TreeImpact, message: &str) {
    record(
        found,
        DiagnosticMetadata {
            phase: DiagnosticPhase::Validation,
            impact,
            code: DiagnosticCode("fixture.code"),
        },
        ReportText(message),
        Offset(0),
        Offset(1),
    );
}

/// Two reports are listed in order with their metadata, and clearing leaves none.
fn reports_are_listed_in_order_and_cleared() -> usize {
    let mut found = Diagnostics::default();
    reported(&mut found, TreeImpact::Preserved, "first");
    reported(&mut found, TreeImpact::Incomplete, "second");
    let listing = listed(&found).collect::<Vec<_>>();
    let kept = listing.len() == 2
        && listing[0].message.0 == b"first"
        && listing[1].message.0 == b"second"
        && listing[1].code == DiagnosticCode("fixture.code")
        && count(&found).0 == 2;
    clear(&mut found);
    usize::from(kept && count(&found).0 == 0 && listed(&found).next().is_none())
}

/// Only a report that keeps the tree whole leaves the syntax complete, and only an incomplete one leaves the lexical stream so.
fn completeness_follows_the_impact_of_the_reports() -> usize {
    let mut found = Diagnostics::default();
    let empty = incomplete_syntax(&found) == SyntaxCompleteness::Complete;
    reported(&mut found, TreeImpact::Preserved, "kept");
    let preserved = incomplete_syntax(&found) == SyntaxCompleteness::Complete
        && incomplete_lexical_stream(&found) == SyntaxCompleteness::Complete;
    reported(&mut found, TreeImpact::LexicallyComplete, "recovered");
    let lexical = incomplete_syntax(&found) == SyntaxCompleteness::Incomplete
        && incomplete_lexical_stream(&found) == SyntaxCompleteness::Complete;
    reported(&mut found, TreeImpact::Incomplete, "lost");
    let lost = incomplete_lexical_stream(&found) == SyntaxCompleteness::Incomplete;
    usize::from(empty && preserved && lexical && lost)
}

/// A phase and an impact are known to queries by their names, and a report without a code has the unspecified one.
fn phases_and_impacts_have_their_query_names() -> usize {
    usize::from(
        phase_name(DiagnosticPhase::Parse).0 == "parse"
            && phase_name(DiagnosticPhase::Validation).0 == "validation"
            && impact_name(TreeImpact::Preserved).0 == "preserved"
            && impact_name(TreeImpact::LexicallyComplete).0 == "lexically-complete"
            && impact_name(TreeImpact::Incomplete).0 == "incomplete"
            && UNSPECIFIED_CODE.0 == "unspecified",
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reports_are_listed_in_order_and_cleared"),
        word: Word::Succeeds,
        input: || reports_are_listed_in_order_and_cleared,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("completeness_follows_the_impact_of_the_reports"),
        word: Word::Succeeds,
        input: || completeness_follows_the_impact_of_the_reports,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("phases_and_impacts_have_their_query_names"),
        word: Word::Succeeds,
        input: || phases_and_impacts_have_their_query_names,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
