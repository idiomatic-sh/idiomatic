//! A file of the ignore corpus adapter's test support: the adapter over a root, and the entry a fixture requires to be observed.

use crate::outbound::corpus::adapters::ignore::adapter::CorpusIgnoreAdapter;
use crate::outbound::corpus::types::SkippedDirectories;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, CorpusListing};

/// The adapter that walks the root and is given nothing.
pub(crate) fn walking(root: RootPath) -> CorpusIgnoreAdapter {
    CorpusIgnoreAdapter::new(
        root,
        CorpusListing::Walked,
        Vec::new(),
        SkippedDirectories::default(),
    )
}

/// Tests that require physical metadata explicitly reject unavailable input fixtures.
pub(crate) fn known_entry(input: &CorpusInput) -> &CorpusEntry {
    let CorpusInput::Available(entry) = input else {
        panic!("this fixture requires observed file metadata");
    };
    entry
}
