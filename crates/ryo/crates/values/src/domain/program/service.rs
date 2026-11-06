//! Checked construction and accessors for query source bundles.

use std::{collections::BTreeMap, sync::Arc};

use super::errors::ProgramError;
use super::types::{ModuleCatalog, QueryBundle, QueryDocumentSource, QuerySourceId, QuerySpelling, SourceText};

/// Validate and construct a stable source identifier.
///
/// # Errors
///
/// Returns [`ProgramError::BlankSourceId`] when the identifier is blank.
pub fn source_id(text: QuerySpelling<'_>) -> Result<QuerySourceId, ProgramError> {
    if text.0.trim().is_empty() {
        return Err(ProgramError::BlankSourceId);
    }
    Ok(QuerySourceId(text.0.into()))
}

/// Construct a source record after its identity has been checked.
#[must_use]
pub fn document_source(id: QuerySourceId, text: impl Into<SourceText>) -> QueryDocumentSource {
    QueryDocumentSource {
        id,
        text: text.into(),
    }
}

/// Validate and index a root source with its modules.
///
/// # Errors
///
/// Returns [`ProgramError::DuplicateSourceId`] if a module repeats the root id or another module id.
pub fn query_bundle(
    root: QueryDocumentSource,
    modules: impl IntoIterator<Item = QueryDocumentSource>,
) -> Result<QueryBundle, ProgramError> {
    let mut indexed = BTreeMap::new();
    for module in modules {
        let id = module.id.clone();
        if id == root.id || indexed.insert(id.clone(), module).is_some() {
            return Err(ProgramError::DuplicateSourceId { id });
        }
    }
    Ok(QueryBundle {
        root,
        modules: Arc::new(indexed),
    })
}

/// Validate module identities once and retain their exact immutable source bytes.
///
/// # Errors
/// Returns [`ProgramError::DuplicateSourceId`] for repeated module ids.
pub fn module_catalog(
    modules: impl IntoIterator<Item = QueryDocumentSource>,
) -> Result<ModuleCatalog, ProgramError> {
    let mut indexed = BTreeMap::new();
    for module in modules {
        let id = module.id.clone();
        if indexed.insert(id.clone(), module).is_some() {
            return Err(ProgramError::DuplicateSourceId { id });
        }
    }
    Ok(ModuleCatalog(Arc::new(indexed)))
}

/// Associate a root with an immutable catalog without copying its documents.
///
/// # Errors
/// Returns [`ProgramError::DuplicateSourceId`] when the root id is a module id.
pub fn query_bundle_with_catalog(
    root: QueryDocumentSource,
    ModuleCatalog(modules): ModuleCatalog,
) -> Result<QueryBundle, ProgramError> {
    if modules.contains_key(&root.id) {
        return Err(ProgramError::DuplicateSourceId { id: root.id });
    }
    Ok(QueryBundle { root, modules })
}

/// Construct a standalone root source.
///
/// # Errors
///
/// Returns [`ProgramError::BlankRootText`] when the query is blank.
pub fn standalone(text: QuerySpelling<'_>) -> Result<QueryBundle, ProgramError> {
    if text.0.trim().is_empty() {
        return Err(ProgramError::BlankRootText);
    }
    let id = source_id(QuerySpelling(super::constants::STANDALONE_SOURCE_ID))?;
    query_bundle(document_source(id, text.0), [])
}

/// The root source's exact text.
#[must_use]
pub fn root_text(bundle: &QueryBundle) -> &SourceText {
    &bundle.root.text
}
