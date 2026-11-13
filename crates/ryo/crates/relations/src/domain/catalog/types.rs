//! The types of the `catalog` concept: what the declaration of a relation says of it beyond its columns: its scope, whether it needs
//! the languages' notes, and the optional capability a language promises it by.

/// Which files a relation's rows are about: each file's own, or the project's as a whole.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::RowScope;
/// use ryo_relations::domain::catalog::service::scope_of;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(scope_of(Relation::Function), RowScope::File);
/// assert_eq!(scope_of(Relation::PackageDependency), RowScope::Project);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowScope {
    /// Its rows are read for one file at a time.
    File,
    /// Its rows are read once, for the project.
    Project,
}

/// Whether a relation's rows are made from the notes the languages write of the files, for the project to read.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::RelationNotes;
/// use ryo_relations::domain::catalog::service::needs_notes;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(needs_notes(Relation::SourceDependency), RelationNotes::Needed);
/// assert_eq!(needs_notes(Relation::Function), RelationNotes::Unneeded);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationNotes {
    /// The languages write their notes when a plan reads it.
    Needed,
    /// It needs no notes.
    Unneeded,
}

/// The capability enum, generated from the relations the catalog declares `optional`.
macro_rules! capability_enum {
    ( $( $variant:ident )* ) => {
        /// Opt-in semantic promises beyond the required language relation baseline: one for each relation the catalog declares
        /// `optional`, named as the relation is.
        ///
        /// # Examples
        ///
        /// ```
        /// use ryo_relations::domain::catalog::types::OptionalCapability;
        /// use ryo_relations::domain::relations::service::{capability_relation, optional_capability};
        /// use ryo_relations::domain::relations::types::Relation;
        ///
        /// assert_eq!(capability_relation(OptionalCapability::BindingRead), Relation::BindingRead);
        /// assert_eq!(optional_capability(Relation::Function), None);
        /// ```
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum OptionalCapability {
            $(
                /// The promise to answer the relation of the same name.
                $variant,
            )*
        }
    };
}

crate::each_relation!(@declared crate::optional_relations, capability_enum);
