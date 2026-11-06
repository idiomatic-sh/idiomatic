//! The one route of a region: chosen once, at compile time, by one rule.
//!
//! Every family analyser leaves its eligible alternative in [`Candidates`]; the rule below
//! keeps exactly one of them as the region's [`Route`]. A route that cannot run declines
//! before any observable work and the instruction is then evaluated by the generic
//! definition of its operation; no second specialised route is ever tried.
use super::{column::ColumnAlternative, selection::SelectionAlternative, types::OrderedExistence};
use crate::domain::lowering::types::LoweredShape;
use std::sync::Arc;

/// How a region is evaluated. Exactly one per region.
#[derive(Clone, Debug, Default)]
pub(crate) enum Route {
    /// The registered operator of each instruction, in order: the definition of the operation.
    #[default]
    Generic,
    /// The reviewed ordered-existence replacement of a self-join anti-join select.
    OrderedExistence(OrderedExistence),
    /// The column kernel over a stored equality index, replacing a join-filter-project chain.
    Column(ColumnAlternative),
    /// The stored selection of a direct select or project over one relation.
    Selection(SelectionAlternative),
    /// The native positions of a step that reads only the tree.
    NativePositions(Arc<LoweredShape>),
}

/// A route without its payload: what a permit is issued for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteKind {
    Generic,
    OrderedExistence,
    Column,
    Selection,
    NativePositions,
}

/// What the family analysers found eligible for one region.
#[derive(Clone, Debug, Default)]
pub(crate) struct Candidates {
    pub ordered: Option<OrderedExistence>,
    pub column: Option<ColumnAlternative>,
    pub selection: Option<SelectionAlternative>,
    pub native: Option<Arc<LoweredShape>>,
}

impl Candidates {
    /// The one selection rule: ordered existence, then the column kernel, then the stored
    /// selection, then native positions; the generic route when none is eligible.
    pub(crate) fn route(self) -> Route {
        if let Some(shape) = self.ordered {
            Route::OrderedExistence(shape)
        } else if let Some(shape) = self.column {
            Route::Column(shape)
        } else if let Some(shape) = self.selection {
            Route::Selection(shape)
        } else if let Some(shape) = self.native {
            Route::NativePositions(shape)
        } else {
            Route::Generic
        }
    }
}

impl Route {
    pub(crate) fn kind(&self) -> RouteKind {
        match self {
            Self::Generic => RouteKind::Generic,
            Self::OrderedExistence(_) => RouteKind::OrderedExistence,
            Self::Column(_) => RouteKind::Column,
            Self::Selection(_) => RouteKind::Selection,
            Self::NativePositions(_) => RouteKind::NativePositions,
        }
    }
    pub(crate) fn ordered_existence(&self) -> Option<&OrderedExistence> {
        match self {
            Self::OrderedExistence(shape) => Some(shape),
            _ => None,
        }
    }
    pub(crate) fn column(&self) -> Option<&ColumnAlternative> {
        match self {
            Self::Column(shape) => Some(shape),
            _ => None,
        }
    }
    pub(crate) fn selection(&self) -> Option<&SelectionAlternative> {
        match self {
            Self::Selection(shape) => Some(shape),
            _ => None,
        }
    }
    pub(crate) fn native_positions(&self) -> Option<&Arc<LoweredShape>> {
        match self {
            Self::NativePositions(shape) => Some(shape),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "route_test.rs"]
mod tests;
