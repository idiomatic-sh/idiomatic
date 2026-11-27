//! Extract the few non-executable rule facts once while plans are transient.
use super::types::{
    CrossFileSteps, FeatureSlot, FeatureSource, FrontierFeed, ProjectRead, RuleRunMetadata,
    StepMetadata,
};
use crate::domain::plan_parts::types::StepScope;
use crate::domain::plans::service::{comparisons_of, kind_of};
use crate::domain::run_metadata::types::Feature;
use crate::domain::{
    plan_parts::types::Input,
    plans::{
        service::{inputs_of, named},
        types::Operation,
    },
    query_plans::types::Plan,
};
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;


