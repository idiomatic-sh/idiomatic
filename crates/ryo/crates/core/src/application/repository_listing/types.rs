use crate::outbound::lint_store::interface::ReadStore;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::MemoryBytes;
use ryo_values::domain::stores::types::Stored;
use std::sync::Arc;

/// The ordered inventory, previous store and budget acquired for one request.
pub(crate) struct RepositoryListing {
    pub files: Vec<CorpusInput>,
    pub stored: Stored,
    pub read: Option<Arc<dyn ReadStore>>,
    pub budget: MemoryBytes,
}
