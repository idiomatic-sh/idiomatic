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

/// The memory workloads: the peak of the command line's cold `check` over `crates/ryo`, where `/usr/bin/time` reports it.
pub(crate) fn memory() -> Vec<crate::measure::Measured> {
    if cfg!(target_os = "linux") {
        vec![check::bench::cold_peak()]
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
