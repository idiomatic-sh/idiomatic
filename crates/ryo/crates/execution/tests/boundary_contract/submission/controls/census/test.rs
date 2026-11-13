//! Each entry trait method runs its refusal control, over intent inventories that list every variant.
use crate::submission::controls::fixtures::*;
use crate::submission::inventory::fixtures::{
    CAPTURES, DELIVERIES, DEMANDS, REASONS, REFUSALS, capture_place, delivery_place, demand_place,
    every_intent, intent_place, places, reason_place, refusal_place,
};
use crate::submission::witness::fixtures::{ALL, Method, Witness, implements_every_entry_trait};
use std::collections::BTreeSet;

#[test]
pub(super) fn every_entry_method_refuses_an_incomplete_extra_or_undeclared_root() {
    implements_every_entry_trait(&Witness);
    assert_eq!(
        ALL.iter().collect::<BTreeSet<_>>().len(),
        ALL.len(),
        "each entry method has exactly one control"
    );
    assert_eq!(
        places(&DELIVERIES, delivery_place),
        (0..DELIVERIES.len()).collect::<BTreeSet<_>>(),
        "DiagnosticDelivery lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&DEMANDS, demand_place),
        (0..DEMANDS.len()).collect::<BTreeSet<_>>(),
        "ObservationDemand lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&CAPTURES, capture_place),
        (0..CAPTURES.len()).collect::<BTreeSet<_>>(),
        "CaptureIntent lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&REFUSALS, refusal_place),
        (0..REFUSALS.len()).collect::<BTreeSet<_>>(),
        "ApplicabilityRefusal lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&REASONS, reason_place),
        (0..REASONS.len()).collect::<BTreeSet<_>>(),
        "ApplicabilityReason lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&every_intent(), intent_place),
        (0..every_intent().len()).collect::<BTreeSet<_>>(),
        "FileRuleIntent lists each variant once at its place: add or remove its submission control"
    );
    let (mut refused, mut exempt) = (0, 0);
    let mut answered = vec![];
    for &method in ALL {
        match control(method) {
            Control::Refuses(refuses) => {
                answered.push((method, refuses()));
                refused += 1;
            }
            Control::AcceptsNoRoot(reason) => {
                assert!(!reason.is_empty(), "{method:?} states why it takes no root");
                exempt += 1;
            }
        }
    }
    assert_eq!(
        refused + exempt,
        ALL.len(),
        "every entry method was classified"
    );
    let expected = ALL
        .iter()
        .filter_map(|&method| answers(method).map(|answers| (method, answers)))
        .collect::<Vec<_>>();
    assert_eq!(
        answered
            .iter()
            .map(|(method, probes)| (
                *method,
                probes
                    .iter()
                    .map(|(what, answer)| (what.as_str(), answer.as_str()))
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|(method, answers)| (*method, answers.to_vec()))
            .collect::<Vec<_>>(),
        "each refusing control's attempts and the answers they met"
    );
}

/// The four file-session methods' control: each method that takes a rule index, given an undeclared one.
const UNDECLARED_RULE: &[(&str, &str)] = &[
    ("rule of an undeclared rule", "Err(NotRun)"),
    ("emit_rows of an undeclared rule", "Err(NotRun)"),
    ("capture_frontiers of an undeclared rule", "Err(NotRun)"),
    ("commit_frontier of an undeclared rule", "Err(NotRun)"),
];

/// What each refusing method's control gives it, and the answer it meets, as debug text; `None` for a method that takes no
/// root.
fn answers(method: Method) -> Option<&'static [(&'static str, &'static str)]> {
    match method {
        Method::UseCasePrepare => Some(&[
            ("a complete declared set", "Ok(())"),
            (
                "a planned rule missing from the set",
                "Err(ObservationConfiguration(InvalidReference))",
            ),
            (
                "a rule no plan answers",
                "Err(ObservationConfiguration(InvalidReference))",
            ),
        ]),
        Method::RequestObservations => Some(&[
            ("the stage of an unnamed program", "Err(NotRun)"),
            (
                "a stage of a named program with no declared occurrence",
                "Ok(Ok(()))",
            ),
        ]),
        Method::RequestOpenFile => Some(&[
            ("0 intents of one kind for 2 rules", "Err(ShapeMismatch)"),
            ("1 intents of one kind for 2 rules", "Err(ShapeMismatch)"),
            ("2 intents of one kind for 2 rules", "Ok(())"),
            ("3 intents of one kind for 2 rules", "Err(ShapeMismatch)"),
        ]),
        Method::RequestRunFile => Some(&[
            (
                "0 intents of one kind for 2 rules",
                "Some(Operating(ShapeMismatch))",
            ),
            (
                "1 intents of one kind for 2 rules",
                "Some(Operating(ShapeMismatch))",
            ),
            (
                "3 intents of one kind for 2 rules",
                "Some(Operating(ShapeMismatch))",
            ),
        ]),
        Method::RootsEvaluateRules => Some(&[
            (
                "rules 0, 0 and the undeclared 3",
                "[Ok((OccurrenceNumber(0), None)), Ok((OccurrenceNumber(1), None)), Err(Operating(NotRun))]",
            ),
            ("the phase, once its occurrences completed", "Ok(())"),
        ]),
        Method::ObservedRunBatches => Some(&[
            ("an occurrence of an undeclared rule", "Err(NotRun)"),
            ("1 sets of inapplicable files for 3 rules", "Err(NotRun)"),
            ("2 sets of inapplicable files for 3 rules", "Err(NotRun)"),
            ("4 sets of inapplicable files for 3 rules", "Err(NotRun)"),
            ("0 sets of inapplicable files for 3 rules", "Ok(())"),
            ("3 sets of inapplicable files for 3 rules", "Ok(())"),
        ]),
        Method::UseCaseExecution
        | Method::FileEmitRows
        | Method::FileRule
        | Method::FileCaptureFrontiers
        | Method::FileCommitFrontier => Some(UNDECLARED_RULE),
        Method::RequestProjectPhase
        | Method::FileFinish
        | Method::RequestFinish
        | Method::StageFinish => None,
    }
}
