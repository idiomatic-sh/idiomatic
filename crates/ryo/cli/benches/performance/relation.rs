//! `relation`: one bench module per command.

mod describe;
mod list;

/// Every command's workload.
pub(crate) fn measured() -> Vec<crate::measure::Measured> {
    vec![describe::bench::workload(), list::bench::workload()]
}
