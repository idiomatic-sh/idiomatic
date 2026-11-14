//! Visits of files of a fake corpus, the way a lint makes them, for the cases of `file_analysis`.

use crate::application::file_analysis::errors::FileAnalysisError;
use crate::application::file_analysis::fixtures::visit::visited;
use crate::application::file_analysis::fixtures::visit::visited_as;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::language::fixtures::fake::LanguageFake;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Visit a real fake file whose injected language emits an invalid node reference, under one rule that reads what the language
/// writes, so the language is asked for it.
fn invalid_injected_provider_rows_prevent_an_analysis_result() -> Result<String, FileAnalysisError>
{
    use crate::application::file_analysis::fixtures::visit::visited_inputs_with_rule_demands;
    use crate::outbound::corpus::fixtures::fake::CorpusFake;
    use crate::outbound::corpus::interface::CorpusPort as _;
    use ryo_execution::inbound::use_cases::metadata::interface::{Mask, Needs, RelationRead};
    use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
    use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
    use ryo_execution_runtime::outbound::facts::interface::FactsPort as _;
    use ryo_language::domain::files::types::FilePath;
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::files::types::{CorpusFile, FileBytes};
    use ryo_values::domain::runs::types::RuleId;
    let mut seen: Vec<String> = Vec::new();
    let corpus = Arc::new(CorpusFake {
        files: vec![CorpusFile {
            executable: Default::default(),
            path: FilePath("invalid-provider.rs".into()),
            bytes: FileBytes(b"fn main() {}".to_vec()),
        }],
        ..CorpusFake::default()
    });
    let files = corpus.inventory()?;
    let (worker, language) = visited_inputs_with_rule_demands(
        corpus,
        &files.0,
        &[Relation::Function],
        LanguageFake {
            out_of_range: true,
            ..LanguageFake::default()
        },
        &crate::domain::render::types::Answering::Findings,
        crate::domain::visit::types::Keeping::Keeps,
        Arc::new(DigestFake),
        FactsFake::default().worker()?,
        None,
        1,
        &[RuleId("reads-functions".into())],
        &[Needs {
            reads: vec![RelationRead {
                relation: Relation::Function,
                kinds: vec![],
            }],
            ..Needs::default()
        }],
        Mask(vec![1]),
        &[ryo_values::domain::runs::types::RuleApplicability::Always],
    )?;
    seen.push(format!("{:?}", language.parses.load(Ordering::Relaxed)));
    seen.push(format!(
        "{:?}",
        matches!(&worker.failure, Some(FileAnalysisError::Provider { provider, .. }) if provider == "fake ")
    ));
    seen.push(format!("{:?}", worker.failure));
    Ok(seen.join("|"))
}

/// Visit a source file, a manifest the language reads and a stray file no language reads: whether the one file left unread is the stray one.
fn a_lint_leaves_unread_the_stray_file_and_not_the_manifest_a_language_reads()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
    let (worker, _) = visited(
        &["a.rs", "project.manifest.fake", "stray.xyz"],
        LanguageFake::default(),
    )?;
    seen.push(format!(
        "{:?}",
        usize::from(
            worker.failure.is_none()
                && worker.tables.unread.len() == 1
                && worker
                    .tables
                    .unread
                    .iter()
                    .all(|file| &*file.path.0 == "stray.xyz"),
        )
    ));
    Ok(seen.join("|"))
}

/// How many times a run that keeps its findings digests two files read over a store that keeps nothing (tens), and over one that
/// keeps (ones, at least one): none, then some.
fn a_run_whose_store_keeps_nothing_digests_no_file() -> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
    let mut counts = Vec::new();
    for keeping in [
        crate::domain::visit::types::Keeping::Discards,
        crate::domain::visit::types::Keeping::Keeps,
    ] {
        let digest = Arc::new(
            ryo_execution_runtime::outbound::digest::fixtures::fake::DigestCounting::default(),
        );
        let (worker, _) = visited_as(
            &["a.rs", "b.rs"],
            LanguageFake::default(),
            &crate::domain::render::types::Answering::Findings,
            keeping,
            Arc::<ryo_execution_runtime::outbound::digest::fixtures::fake::DigestCounting>::clone(
                &digest,
            ),
        )?;
        seen.push(format!("{:?}", worker.failure.is_none()));
        seen.push(format!("{:?}", worker.tables.fresh.len()));
        counts.push(digest.digests.load(Ordering::Relaxed));
    }
    seen.push(format!(
        "{:?}",
        counts.first().copied().unwrap_or(1) * 10
            + usize::from(counts.get(1).is_some_and(|each| *each > 0))
    ));
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `invalid_injected_provider_rows_prevent_an_analysis_result` scenario.
    InvalidInjectedProviderRowsPreventAnAnalysisResult,
    /// The `a_lint_leaves_unread_the_stray_file_and_not_the_manifest_a_language_reads` scenario.
    ALintLeavesUnreadTheStrayFileAndNotTheManifestALanguageReads,
    /// The `a_run_whose_store_keeps_nothing_digests_no_file` scenario.
    ARunWhoseStoreKeepsNothingDigestsNoFile,
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
        name: CaseName("invalid_injected_provider_rows_prevent_an_analysis_result"),
        word: Word::Succeeds,
        input: || Given::InvalidInjectedProviderRowsPreventAnAnalysisResult,
        expect: Expect::Value(
            "1|true|Some(Provider { provider: \"fake \", code: NodeOutsideTable })",
        ),
    },
    Case {
        name: CaseName("a_lint_leaves_unread_the_stray_file_and_not_the_manifest_a_language_reads"),
        word: Word::Succeeds,
        input: || Given::ALintLeavesUnreadTheStrayFileAndNotTheManifestALanguageReads,
        expect: Expect::Value("1"),
    },
    Case {
        name: CaseName("a_run_whose_store_keeps_nothing_digests_no_file"),
        word: Word::Succeeds,
        input: || Given::ARunWhoseStoreKeepsNothingDigestsNoFile,
        expect: Expect::Value("true|2|true|2|1"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::InvalidInjectedProviderRowsPreventAnAnalysisResult => {
            invalid_injected_provider_rows_prevent_an_analysis_result()?
        }
        Given::ALintLeavesUnreadTheStrayFileAndNotTheManifestALanguageReads => {
            a_lint_leaves_unread_the_stray_file_and_not_the_manifest_a_language_reads()?
        }
        Given::ARunWhoseStoreKeepsNothingDigestsNoFile => {
            a_run_whose_store_keeps_nothing_digests_no_file()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
