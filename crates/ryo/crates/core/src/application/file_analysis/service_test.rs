//! Analysis uses its injected language and propagates required failures.
use super::errors::FileAnalysisError;
use super::service::FileAnalysis;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Selection;
use crate::cases::Word;
use crate::cases::run;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::types::ProjectNotes;
use crate::outbound::language::types::ProjectToRead;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::project::types::ManifestFile;
use ryo_language::inbound::use_cases::project::types::ManifestText;
use ryo_language::inbound::use_cases::project::types::RootPath;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[cfg(test)]
mod capture;
#[cfg(test)]
mod failure;
#[cfg(test)]
mod gating;
#[cfg(test)]
mod manifest;
#[cfg(test)]
mod recovery;
#[cfg(test)]
mod replay;
#[cfg(test)]
mod resident;
#[cfg(test)]
mod visit;

type Input = &'static str;
/// The projections the injected language made, and the manifests the projection refused.
type Value = [usize; 2];
type Failure = FileAnalysisError;

fn subject(path: Input) -> Result<Value, Failure> {
    if path == "unlinked-source-provider" {
        let service = FileAnalysis::new(
            Arc::new(CorpusFake::default()),
            Arc::new(LanguageFake::default()),
            Arc::new(DigestFake),
            Arc::new(MemoryFake::default()),
        );
        service.validate_source_language(Some(
            &ryo_values::domain::descriptions::types::LanguageName("not-linked".into()),
        ))?;
        return Ok([0, 0]);
    }
    let language = Arc::new(LanguageFake::default());
    let service = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::<LanguageFake>::clone(&language),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let input = ProjectToRead {
        search_scope: &Default::default(),
        given_inventory: &Default::default(),
        observed_entries: &Default::default(),
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        root: &RootPath(".".into()),
        manifests: &[ManifestFile {
            path: FilePath(path.into()),
            text: ManifestText("".into()),
        }],
        files: &[],
        notes: &ProjectNotes(Vec::new()),
        wanted: &[],
    };
    let mut facts = FactsFake::default().worker()?;
    let projected = service.project_now(&input, &mut Symbols::default(), &mut *facts)?;
    Ok([
        language.projects.load(Ordering::Relaxed),
        projected.refused.len(),
    ])
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_source_provider_no_language_links_is_refused_before_any_file_is_read"),
        word: Word::Refuses,
        input: || "unlinked-source-provider",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::SourceLanguage { name } if name == "not-linked") {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_injected_language_performs_project_analysis"),
        word: Word::Succeeds,
        input: || "manifest.fake",
        expect: Expect::Value([1, 0]),
    },
    Case {
        name: CaseName("a_required_language_failure_prevents_an_analysis_result"),
        word: Word::Fails,
        input: || "broken.manifest.fake",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::Language(_)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
