//! The `repository` service: free functions named by verb over the files of a run, with no IO and no ports.

use crate::domain::keeping::service::configuration_bytes;
use crate::domain::keeping::types::{Configuration, DigestInput, HostedText};
use crate::domain::rules::types::RuleSpec;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::CorpusEntry;

/// What is read of the rules and the relations the host declares (as their lines, spelled once per run however many rules
/// are digested) to say whether what the store kept of the rules can be used, as the bytes to digest.
#[must_use]
pub(crate) fn configuration(rules: &[RuleSpec], hosted: &[HostedText]) -> DigestInput {
    configuration_bytes(&Configuration { rules, hosted })
}

/// Puts the files in the order of their paths, which is the order their findings leave in: a file is numbered by its place in it.
pub(crate) fn by_path(files: &mut [CorpusEntry]) {
    files.sort_by(|left, right| left.path.cmp(&right.path));
}

/// The paths of the files, as the tables of the run spell them, in the order of the files.
#[must_use]
pub(crate) fn paths_of(files: &[CorpusEntry]) -> Vec<FilePath> {
    files
        .iter()
        .map(|file| FilePath(file.path.0.clone()))
        .collect()
}
