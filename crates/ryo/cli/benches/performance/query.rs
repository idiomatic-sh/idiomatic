//! `query`: one bench module per command.

mod run;
mod validate;

/// Every command's workload.
pub(crate) fn measured() -> Vec<crate::measure::Measured> {
    vec![run::bench::workload(), validate::bench::workload()]
}
