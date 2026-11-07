//! `ratchet`: one bench module per command.

mod check;
mod down;
mod init;
mod up;

/// Every command's workload.
///
/// # Errors
///
/// The first error a workload's setup or run answers.
pub(crate) fn measured() -> Result<Vec<crate::measure::Measured>, Box<dyn std::error::Error>> {
    Ok(vec![
        check::bench::workload()?,
        down::bench::workload()?,
        init::bench::workload()?,
        up::bench::workload()?,
    ])
}
