//! The cases of consulted executable metadata, cold, warm and edited runs, skipped syntax and removed files.
use super::super::interface::LintUseCase as _;
use crate::cases::run;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::runs::types::AnswerShape;
use std::sync::Arc;
use super::given::Given;
use super::given::SOURCES;
use super::given::request;
use super::lifecycle::counted_lint;
use super::lifecycle::reading_the_project;
use super::stores::THREE;
use super::stores::answered;
use super::stores::store_of;
use super::super::errors::LintError;
use super::worlds::lint;
use super::worlds::world;
use crate::inbound::use_cases::lint::fixtures::given::violations;

pub(crate) fn consulted_equal_byte_files_revalidate_on_each_executable_metadata_change() -> Result<(usize, usize), LintError> {
    use ryo_values::domain::files::types::ExecutablePermission;
    for before in [false, true] {
        for changed in [false, true] {
            let store = Arc::new(LintStoreFake::default());
            let given = |executable| Given {
                files: vec![
                    SOURCES[0],
                    SOURCES[1],
                    ("project.manifest.fake", "members = 1"),
                ],
                outside: vec![("../config.fake", "target = 1")],
                outside_executable: if executable {
                    ExecutablePermission::PosixExecutable
                } else {
                    ExecutablePermission::PosixNotExecutable
                },
                store: Arc::clone(&store),
                root: "root",
                query: Arc::new(reading_the_project()),
                language: LanguageFake {
                    consulted: vec![FilePath("../config.fake".into())],
                    ..LanguageFake::default()
                },
                ..violations()
            };
            assert_eq!(counted_lint(given(before)).unwrap().0, 1);
            assert_eq!(
                counted_lint(given(if changed { !before } else { before }))
                    .unwrap()
                    .0,
                usize::from(changed),
                "consulted metadata must invalidate even when source digest is equal"
            );
        }
    }
    Ok((1, 0))
}

/// A cold, a warm and an edited run answer the rows and findings a cold run over the same files answers, whichever store keeps
/// them; the stores that keep read only the files that changed, the one that discards reads every file each time.
pub(crate) fn cold_warm_and_edited_runs_answer_the_same_in_every_store() -> Result<(usize, usize), LintError> {
    use ryo_values::domain::runs::types::RuleApplicability::Always;
    let edited = [("a.rs", "fn changed() {}"), THREE[1], THREE[2]];
    for answer in [AnswerShape::Findings, AnswerShape::Rows(RowTotal(20))] {
        for (kind, parsed) in [
            ("memory", [3, 0, 1]),
            ("postcard", [3, 0, 1]),
            ("discard", [3, 3, 3]),
        ] {
            let directory = tempfile::tempdir().expect("a directory");
            let kept_in = store_of(kind, directory.path());
            for (run, files) in [&THREE[..], &THREE[..], &edited[..]]
                .into_iter()
                .enumerate()
            {
                let oracle = answered(
                    files,
                    &[],
                    Always,
                    answer,
                    store_of("discard", directory.path()),
                );
                let now = answered(files, &[], Always, answer, Arc::clone(&kept_in));
                assert_eq!(
                    now.rows, oracle.rows,
                    "{kind} {answer:?} run {run}: the rows"
                );
                assert_eq!(
                    now.undecided, oracle.undecided,
                    "{kind} run {run}: what was undecided"
                );
                assert_eq!(
                    now.skipped, oracle.skipped,
                    "{kind} run {run}: what was skipped"
                );
                // The reuse of a rows answer is not asserted: it is decided apart from the findings.
                if answer == AnswerShape::Findings {
                    assert_eq!(now.parses, parsed[run], "{kind} run {run}: the files read");
                }
            }
        }
    }
    Ok((1, 0))
}

/// One file whose syntax a rule skips does not poison the cross-file partitions of the files around it: cold or warm, they answer
/// what the same run without that file answers, and the skip is named and kept.
pub(crate) fn a_syntax_skipped_file_does_not_poison_unrelated_partitions_cold_or_warm() -> Result<(usize, usize), LintError> {
    use ryo_values::domain::runs::types::RuleApplicability::{AcceptedSyntax, Always};
    let without = [THREE[0], THREE[2]];
    for kind in ["memory", "postcard"] {
        let directory = tempfile::tempdir().expect("a directory");
        let oracle = answered(
            &without,
            &[],
            Always,
            AnswerShape::Findings,
            store_of("discard", directory.path()),
        );
        let kept_in = store_of(kind, directory.path());
        let cold = answered(
            &THREE,
            &["b.rs"],
            AcceptedSyntax,
            AnswerShape::Findings,
            Arc::clone(&kept_in),
        );
        let warm = answered(
            &THREE,
            &["b.rs"],
            AcceptedSyntax,
            AnswerShape::Findings,
            kept_in,
        );
        assert_eq!(cold.rows, oracle.rows, "{kind}: cold rows");
        assert_eq!(warm.rows, oracle.rows, "{kind}: warm rows");
        assert_eq!(cold.undecided, "[]", "{kind}: nothing undecided cold");
        assert_eq!(warm.undecided, "[]", "{kind}: nothing undecided warm");
        assert!(
            cold.skipped.contains("b.rs") && cold.skipped.contains("SyntaxRejected"),
            "{kind}: {}",
            cold.skipped
        );
        assert_eq!(warm.skipped, cold.skipped, "{kind}: the skip is kept");
        assert_eq!(warm.parses, 0, "{kind}: a warm run reads no file");
    }
    Ok((1, 0))
}

/// A file whose syntax a rule skips is Skipped with the reason the language gave, for the file-phase rule as for the cross-file one:
/// the rows of the files around it are those of the same run without it, and nothing is undecided; it is never answered complete.
pub(crate) fn a_syntax_rejected_file_is_skipped_with_its_reason_and_never_undecided_or_complete() -> Result<(usize, usize), LintError> {
    use ryo_values::domain::runs::types::RuleApplicability::{AcceptedSyntax, Always};
    let lint = |files: &[(&'static str, &'static str)], rejected: &[&str], applicability| {
        let mut given = Given {
            files: files.to_vec(),
            ..violations()
        };
        given.language.rejected_syntax = rejected
            .iter()
            .map(|path| FilePath((*path).into()))
            .collect();
        let mut asked = request(&given);
        asked.rules[0].applicability = applicability;
        let world = world(given);
        let report = world.use_case.lint(asked).expect("the lint answers");
        let rows = std::mem::take(
            &mut *world
                .collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
        .rows;
        (
            rows,
            format!("{:?}", report.unknowns),
            format!("{:?}", report.skipped),
        )
    };
    let oracle = lint(&[THREE[0], THREE[2]], &[], Always);
    let (rows, undecided, skipped) = lint(&THREE, &["b.rs"], AcceptedSyntax);
    assert_eq!(rows, oracle.0, "the rows of the files around it");
    assert_eq!(undecided, "[]", "a rejected file is not undecided");
    assert!(
        skipped.contains("b.rs") && skipped.contains("SyntaxRejected"),
        "named with its reason: {skipped}"
    );
    assert!(
        oracle.2 == "[]",
        "a file that is read is not skipped: {}",
        oracle.2
    );
    Ok((1, 0))
}

/// A file the store kept a skip or the rows of and the run no longer lists is gone: nothing the store kept of it is an answer about
/// this run, so a warm run without it names it in neither what is undecided nor what is skipped, and answers what a run over a store
/// that keeps nothing answers.
pub(crate) fn removed_file_leaves_nothing() -> Result<(usize, usize), LintError> {
    use ryo_values::domain::runs::types::RuleApplicability::{AcceptedSyntax, Always};
    let without = [THREE[0], THREE[2]];
    for kind in ["memory", "postcard"] {
        for applicability in [AcceptedSyntax, Always] {
            let directory = tempfile::tempdir().expect("a directory");
            let kept_in = store_of(kind, directory.path());
            let cold = answered(
                &THREE,
                &["b.rs"],
                applicability,
                AnswerShape::Findings,
                Arc::clone(&kept_in),
            );
            // The language is the cold run's: it still rejects `b.rs` and accepts the files around it; only the file is gone.
            let oracle = answered(
                &without,
                &["b.rs"],
                applicability,
                AnswerShape::Findings,
                store_of("discard", directory.path()),
            );
            // What the store keeps of the rejected file: a rule that asks for accepted syntax skips it; a rule that always applies
            // answers it as any other file, so its rows are among the cold rows. The fake language names each function by its
            // file's path, so the rejected file's row is the one whose name is `b.rs`.
            let names_b = |row: &crate::outbound::row_sink::types::KeptRow| {
                row.0.iter().any(|cell| {
                    matches!(
                        cell,
                        Some(
                            ryo_values::domain::row_texts::types::Cell::File(text)
                                | ryo_values::domain::row_texts::types::Cell::Text(text)
                        ) if &**text == "b.rs"
                    )
                })
            };
            match applicability {
                AcceptedSyntax => assert!(
                    cold.undecided.contains("b.rs") || cold.skipped.contains("b.rs"),
                    "{kind} {applicability:?}: the cold run records the rejected file: {} {}",
                    cold.undecided,
                    cold.skipped
                ),
                Always => assert!(
                    cold.rows.iter().any(names_b) && !oracle.rows.iter().any(names_b),
                    "{kind} {applicability:?}: the cold run answers the rejected file: {:?}",
                    cold.rows
                ),
            }
            let warm = answered(&without, &["b.rs"], applicability, AnswerShape::Findings, kept_in);
            assert!(
                !warm.undecided.contains("b.rs") && !warm.skipped.contains("b.rs"),
                "{kind} {applicability:?}: the removed file is named: {} {}",
                warm.undecided,
                warm.skipped
            );
            assert_eq!(warm.rows, oracle.rows, "{kind} {applicability:?}: rows");
            assert_eq!(
                warm.undecided, oracle.undecided,
                "{kind} {applicability:?}: undecided"
            );
            assert_eq!(
                warm.skipped, oracle.skipped,
                "{kind} {applicability:?}: skipped"
            );
        }
    }
    Ok((0, 0))
}
