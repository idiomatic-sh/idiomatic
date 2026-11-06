//! The `query_language` domain's rules, one module per concept.

pub(crate) mod assembling;
pub(crate) mod calling;
pub(crate) mod capture_pushing;
pub(crate) mod combining;
pub(crate) mod comparing;
pub(crate) mod cutting;
pub(crate) mod fusing;
pub(crate) mod idioms;
pub(crate) mod inlining;
pub(crate) mod joining;
pub(crate) mod linking;
pub(crate) mod lowering;
pub(crate) mod planning;
pub(crate) mod pruning;
pub(crate) mod pushing;
pub(crate) mod rendering;
pub(crate) mod reshaping;
pub(crate) mod resolving;
pub(crate) mod rewriting;
pub(crate) mod selecting;
pub(crate) mod spanning;
pub(crate) mod syntax;
pub(crate) mod traversing;
pub(crate) mod validation;
pub(crate) mod walking;

pub(crate) mod substituting;

pub(crate) mod observation_inputs;
