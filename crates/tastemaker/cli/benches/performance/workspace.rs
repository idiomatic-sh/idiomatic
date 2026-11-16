//! `workspace`: one bench module per command.

mod add;
mod check;
mod create;
mod explain;
mod init;
mod list;
mod scaffold;
mod update;
mod validate;

/// The workloads taken on four processors where `/usr/bin/time` reports the peak: the command line's cold `check` over `crates/ryo`, and
/// its `validate` over the repository's own definitions, cold and warm.
pub(crate) fn memory() -> Vec<crate::measure::Measured> {
    if cfg!(target_os = "linux") {
        let mut every = vec![check::bench::cold_peak()];
        every.extend(validate::bench::repository());
        every
    } else {
        Vec::new()
    }
}

/// Every command's workload.
pub(crate) fn measured() -> Vec<crate::measure::Measured> {
    vec![
        add::bench::workload(),
        check::bench::workload(),
        check::bench::instances(),
        create::bench::workload(),
        explain::bench::workload(),
        init::bench::workload(),
        list::bench::workload(),
        scaffold::bench::workload(),
        update::bench::workload(),
        validate::bench::workload(),
    ]
}
