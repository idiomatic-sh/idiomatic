//! The types of the `admission` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::FileId;
use std::sync::Arc;

/// The files a rule reads after the files: the request's files and the rule's own evidence of each it skipped, does not apply
/// to or was refused on.
#[derive(Clone)]
pub(crate) struct RequiredPartitions {
    pub(crate) request: Arc<super::super::request_identity::service::RequestIdentity>,
    pub(crate) files: Box<[FileId]>,
    pub(crate) covered: Box<[ryo_execution_domain::domain::coverage::types::Covered]>,
    pub(crate) skipped: Box<[ryo_values::domain::runs::types::Skipped]>,
    pub(crate) inapplicable: Box<[FileId]>,
    pub(crate) refused: Box<[FileId]>,
}

impl PartialEq for RequiredPartitions {
    fn eq(&self, other: &Self) -> bool {
        self.request == other.request
            && self.files == other.files
            && self.covered == other.covered
            && self.skipped == other.skipped
    }
}

impl Eq for RequiredPartitions {}
