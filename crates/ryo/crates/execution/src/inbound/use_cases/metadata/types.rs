//! Data returned by pure metadata decisions; none carries execution authority.

/// The precise partial source set and the declared number of files it may name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AffectedFiles<'a> {
    pub affected: &'a [Option<ryo_relations::domain::relations::types::FileId>],
    pub file_count: CoverageFileCount,
}

/// Number of declared files whose partial sources may be named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverageFileCount(pub usize);

/// Distinct validated file ordinals affected by one partial source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileOrdinals(pub Vec<usize>);

/// A wrapper that names a bare standard type at the use case's boundary, read through `Deref` as the type it holds.
macro_rules! named {
    ($(#[$doc:meta])* $name:ident$(<$life:lifetime>)?($inner:ty)) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name$(<$life>)?(pub $inner);
        impl$(<$life>)? std::ops::Deref for $name$(<$life>)? {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl$(<$life>)? From<$inner> for $name$(<$life>)? {
            fn from(inner: $inner) -> Self {
                Self(inner)
            }
        }
    };
}

named!(
    /// Why a plan's kinds are refused, when they are.
    KindRefusalFound(Option<ryo_execution_domain::domain::kind_check::types::KindRefusal>)
);
named!(
    /// The kinds a plan asks for, with the bindings it was given.
    AskedKinds(Vec<ryo_execution_domain::domain::kind_check::types::AskedKind>)
);
named!(
    /// The parameter bindings of each plan, by plan.
    PlanBindings<'a>(&'a [Vec<ryo_values::domain::query_bindings::types::ParameterBinding>])
);
named!(
    /// The first kind no vocabulary knows, when there is one.
    UnknownKindFound(Option<ryo_execution_domain::domain::kind_check::types::UnknownKind>)
);
named!(
    /// The features the engine reports, in catalog order.
    FeatureCatalog(Vec<ryo_values::domain::run_metadata::types::Feature>)
);
named!(
    /// The rows each step of each rule was fed, by rule.
    FedRows<'a>(&'a [Vec<ryo_execution_domain::domain::operator::types::RowsFed>])
);
named!(
    /// The work of each step of each rule, by rule.
    StepWorks(Vec<Vec<ryo_execution_domain::domain::features::types::StepWork>>)
);
named!(
    /// The rows each feature was fed, by rule.
    FeatureRowsByRule(Vec<ryo_execution_domain::domain::features::types::FeatureRows>)
);
named!(
    /// The relations a source read in part.
    PartialReads(Vec<ryo_execution_domain::domain::coverage::types::Partial>)
);
named!(
    /// The relations a plan's observations read.
    ObservedReads(Vec<ryo_execution_domain::domain::trust::types::RelationRead>)
);
named!(
    /// The unknowns of a project.
    ProjectUnknowns(Vec<ryo_execution_domain::domain::coverage::types::Unknown>)
);
