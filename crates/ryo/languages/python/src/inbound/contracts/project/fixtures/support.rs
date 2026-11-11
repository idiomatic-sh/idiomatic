//! A group of the contract's cases.
//!
//! What the project contract's cases share: the readers of a run with pep621 alone, and the reader that counts its rows.

use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use ryo::languages;
use std::sync::Arc;

/// The readers of a run whose only manifest tool is pep621.
pub(super) fn pep(_: &[languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
    vec![Arc::new(
        crate::outbound::manifest_reading::factory::pep621(),
    )]
}

#[derive(Default)]
pub(super) struct ReaderOutcome {
    pub(super) rows: usize,
    pub(super) partial: Vec<String>,
}
impl<Row> languages::Rows<Row> for ReaderOutcome {
    fn write(&mut self, _row: Row) {
        self.rows += 1;
    }
    fn partial(&mut self, cause: languages::ReportText<'_>, _missing: languages::Missing) {
        self.partial.push(cause.0.to_owned());
    }
    fn partial_at(
        &mut self,
        _file: languages::FileId,
        cause: languages::ReportText<'_>,
        missing: languages::Missing,
    ) {
        <Self as languages::Rows<Row>>::partial(self, cause, missing);
    }
}
