//! What a corpus is measured with, and what its language tells the measurement.
use crate::domain::suite::types::certificate::value::Certificate;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::requests::types::{
    BenchmarkCoverage, CohortSelection, SearchRoots,
};
use ryo_benchmark_contracts::domain::workloads::types::{Project, RuleContextProfile};
use std::path::Path;

/// Which sources of a project are measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceProfile {
    /// Only sources the pinned reference parser accepts.
    Valid,
    /// Every discovered source of the language.
    Full,
}

/// The language as observations name it.
#[derive(Clone, Copy, Debug)]
pub struct LanguageTitle<'a>(pub &'a str);

/// The pinned reference tool as observations name it.
#[derive(Clone, Copy, Debug)]
pub struct ToolTitle<'a>(pub &'a str);

/// The sample name of the reference contestant.
#[derive(Clone, Copy, Debug)]
pub struct SampleLabel<'a>(pub &'a str);

/// The suffix of a file the matched input profile admits.
#[derive(Clone, Copy, Debug)]
pub struct SourceSuffix<'a>(pub &'a str);

/// The input-profile label an observation carries.
#[derive(Clone, Copy, Debug)]
pub struct ProfileLabel<'a>(pub &'a str);

/// The names and suffixes the language gives its sources, its reference and its profiles.
#[derive(Clone, Copy, Debug)]
pub struct CorpusLanguage<'a> {
    /// The language as observations name it.
    pub display: LanguageTitle<'a>,
    /// The pinned reference tool as observations name it.
    pub reference_display: ToolTitle<'a>,
    /// The sample name of the reference contestant.
    pub reference_name: SampleLabel<'a>,
    /// The suffixes of the files the matched input profile admits.
    pub suffixes: &'a [SourceSuffix<'a>],
    /// The reference's rule for a source its parser rejects.
    pub invalid_syntax: &'a ryo::linter::RuleId,
    /// The input-profile label of a run limited to accepted sources.
    pub valid_profile: ProfileLabel<'a>,
    /// The input-profile label of a run over every source.
    pub full_profile: ProfileLabel<'a>,
}

/// Per-rule finding multiplicities, written explicitly rather than as debug representations.
#[derive(Clone, Debug)]
pub(super) struct FindingCounts<'a>(pub std::collections::BTreeMap<&'a ryo::linter::RuleId, usize>);

impl std::fmt::Display for FindingCounts<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, (rule, count)) in self.0.iter().enumerate() {
            if index != 0 {
                f.write_str(super::constants::FINDING_SEPARATOR)?;
            }
            write!(f, "{rule}={count}")?;
        }
        Ok(())
    }
}

/// What one agreement between two reports compares, named for the refusal that reports a difference.
#[derive(Clone, Copy, Debug)]
pub(super) struct ComparisonName(pub &'static str);
