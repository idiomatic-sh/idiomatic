//! Application execution capabilities beside pure domain run configuration.
use super::executor::PrimitiveExecutor;
use crate::domain::operator::contract::types::RequestOperators;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
#[derive(Clone)]
pub(crate) struct VisitSetting<'a> {
    /// Explicit request provider; none preserves ordinary path recognition.
    pub source_language: Option<&'a ryo_values::domain::descriptions::types::LanguageName>,
    pub executor: &'a dyn PrimitiveExecutor,
    pub operators: Arc<RequestOperators>,
    pub domain: crate::domain::workers::types::VisitSetting<'a>,
}
impl<'a> Deref for VisitSetting<'a> {
    type Target = crate::domain::workers::types::VisitSetting<'a>;
    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}
impl DerefMut for VisitSetting<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.domain
    }
}
#[derive(Clone, Debug)]
pub(crate) struct CrossingInput<'a> {
    pub executor: &'a dyn PrimitiveExecutor,
    pub operators: Arc<RequestOperators>,
    pub domain: crate::domain::crossing::types::CrossingInput<'a>,
}
impl<'a> Deref for CrossingInput<'a> {
    type Target = crate::domain::crossing::types::CrossingInput<'a>;
    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}
