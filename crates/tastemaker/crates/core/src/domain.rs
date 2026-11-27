//! The core's rules, one module per concept: pure functions over its values, with no IO and no ports.

pub(crate) mod answering;
pub(crate) mod applying;
pub(crate) mod blocks;
pub(crate) mod built_ins;
pub(crate) mod carrying;
pub(crate) mod contract_rendering;
pub(crate) mod declarations;
pub(crate) mod globs;
pub(crate) mod ignores;
pub(crate) mod keyed;
pub(crate) mod loading;
pub(crate) mod planning;
pub(crate) mod reaching;
pub mod reporting;
pub(crate) mod scaffolding;
pub(crate) mod scenario_proving;
