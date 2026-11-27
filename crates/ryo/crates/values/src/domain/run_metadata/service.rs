//! Extract the few non-executable rule facts once while plans are transient.
use super::types::{
    CrossFileSteps, FeatureSlot, FeatureSource, FrontierFeed, ProjectRead, RuleRunMetadata,
    StepMetadata};
use crate::domain::{
    plan_parts::types::Input,
    plans::{
        service::{},
        types::Operation}};