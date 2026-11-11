//! Each mode of the partition controls through the owner: partial, rekeyed and current publications.
use super::super::fixtures::Mode;
use super::super::test::exercise;
use super::super::test::exercise_with;
use ryo_relations::domain::relations::types::FileId;

#[test]
fn named_absence_keeps_good_file_bags_and_orders_while_bad_file_stays_unknown() {
    for empty in [false, true] {
        for project_away_file in [false, true] {
            for stale_complete in [false, true] {
                exercise(empty, project_away_file, stale_complete, Mode::PartialRight);
            }
        }
    }
}

#[test]
fn partial_left_and_complete_empty_right_preserve_known_positive_rows() {
    for project_away_file in [false, true] {
        for difference in [false, true] {
            exercise(
                false,
                project_away_file,
                false,
                Mode::PartialLeft { difference },
            );
        }
    }
}

#[test]
fn empty_rekeying_join_widens_uncertainty_but_file_equality_keeps_good_files_known() {
    for file_key in [false, true] {
        exercise(true, false, false, Mode::Rekey { file_key });
    }
}

#[test]
fn current_partial_project_publication_retains_owner_provenance_through_real_consumer() {
    for project_away_file in [false, true] {
        exercise(true, project_away_file, false, Mode::Current);
    }
}

#[test]
fn current_project_relation_coverage_precedes_rekeying_and_survives_named_handover() {
    for file_key in [false, true] {
        exercise(true, false, false, Mode::CurrentRekey { file_key });
    }
}

/// Rules A (rule 1, no inapplicable files) and B (rule 2, `inapplicable`) share one project
/// execution; B reads the restored bound observation. The expectations below are the same with
/// and without B's set, so a run whose rules differ in it answers exactly as one whose rules do not.
#[test]
fn a_rule_with_inapplicable_files_reads_a_restored_observation_as_a_rule_without() {
    for (empty, mode) in [
        (true, Mode::PartialRight),
        (false, Mode::PartialLeft { difference: false }),
        (true, Mode::Rekey { file_key: true }),
        (true, Mode::Current),
    ] {
        for inapplicable in [&[][..], &[FileId(0)][..]] {
            exercise_with(empty, false, false, mode, inapplicable);
        }
    }
}
