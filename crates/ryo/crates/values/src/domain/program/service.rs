//! Checked construction and accessors for query source bundles.

use std::collections::BTreeMap;

use crate::domain::plan_parts::types::QueryText;

use super::errors::QueryBundleError;
use super::types::{QueryBundle, QueryDocumentSource, QuerySourceId};

/// Validate and construct a stable source identifier.
///
/// # Errors
///
/// Returns [`QueryBundleError::BlankSourceId`] when the identifier is blank.
pub fn source_id(text: &str) -> Result<QuerySourceId, QueryBundleError> {
    if text.trim().is_empty() {
        return Err(QueryBundleError::BlankSourceId);
    }
    Ok(QuerySourceId(text.into()))
}

/// Construct a source record after its identity has been checked.
#[must_use]
pub fn document_source(id: QuerySourceId, text: impl Into<Box<str>>) -> QueryDocumentSource {
    QueryDocumentSource {
        id,
        text: text.into(),
    }
}

/// Validate and index a root source with its modules.
///
/// # Errors
///
/// Returns [`QueryBundleError::DuplicateSourceId`] if a module repeats the root id or another module id.
pub fn query_bundle(
    root: QueryDocumentSource,
    modules: impl IntoIterator<Item = QueryDocumentSource>,
) -> Result<QueryBundle, QueryBundleError> {
    let mut indexed = BTreeMap::new();
    for module in modules {
        let id = module.id.clone();
        if id == root.id || indexed.insert(id.clone(), module).is_some() {
            return Err(QueryBundleError::DuplicateSourceId { id });
        }
    }
    Ok(QueryBundle {
        root,
        modules: indexed,
    })
}

/// Construct a standalone root source.
///
/// # Errors
///
/// Returns [`QueryBundleError::BlankRootText`] when the query is blank.
pub fn standalone(text: &str) -> Result<QueryBundle, QueryBundleError> {
    if text.trim().is_empty() {
        return Err(QueryBundleError::BlankRootText);
    }
    let id = source_id("<query>")?;
    query_bundle(document_source(id, text), [])
}

/// The root source's exact text.
#[must_use]
pub fn root_text(bundle: &QueryBundle) -> &str {
    &bundle.root.text
}

impl From<QueryText> for QueryBundle {
    fn from(text: QueryText) -> Self {
        let root = QueryDocumentSource {
            id: QuerySourceId("<query>".into()),
            text: text.0,
        };
        Self {
            root,
            modules: BTreeMap::new(),
        }
    }
}
