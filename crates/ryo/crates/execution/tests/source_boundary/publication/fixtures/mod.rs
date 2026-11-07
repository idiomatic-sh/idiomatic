//! Probe source is inserted into the actual production crate, never a copied model.
pub(super) const DETACHED_RECORD: &str = r#"
mod detached_publication_record {
    use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
    use crate::domain::operating::errors::OperatingError;
    use std::sync::Arc;
    fn escape(owner: &RelationOwner, publication: PublishedInput) -> Result<(), OperatingError> {
        let detached = Arc::clone(publication.record());
        drop(publication);
        let rows = owner.resolve(detached.as_ref(), &|_| Err(OperatingError::NotRead))?;
        drop(rows);
        Ok(())
    }
    const _: fn(&RelationOwner, PublishedInput) -> Result<(), OperatingError> = escape;
}
"#;
pub(super) const BORROWED_RECORD: &str = r#"
mod borrowed_publication_record {
    use crate::application::file_dag::relations::{handle::service::PublishedInput, publication::types::RelationResult};
    fn escape(publication: PublishedInput) -> &'static RelationResult { publication.as_ref() }
    const _: fn(PublishedInput) -> &'static RelationResult = escape;
}
"#;
pub(super) const LEASED_CLONE: &str = r#"
mod leased_publication_clone {
    use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput, publication::types::RelationResult};
    use crate::domain::operating::errors::OperatingError;
    fn read(owner: &RelationOwner, publication: PublishedInput) -> Result<(), OperatingError> {
        let alias = publication.clone();
        drop(publication);
        let borrowed: &RelationResult = &alias;
        assert!(std::ptr::eq(borrowed, alias.as_ref()));
        let rows = owner.resolve(alias.as_ref(), &|_| Err(OperatingError::NotRead))?;
        drop(rows);
        drop(alias);
        Ok(())
    }
    const _: fn(&RelationOwner, PublishedInput) -> Result<(), OperatingError> = read;
}
"#;
