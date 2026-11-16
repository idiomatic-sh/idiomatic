//! Reads this process's CPU clock and `/proc/self` memory figures, and decides nothing.
use super::constants;
use crate::outbound::process_meter::interface::ProcessMeterPort;
use crate::outbound::process_meter::types::CpuTime;
use ryo_benchmark_contracts::domain::measurements::types::ResidentBytes;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// The meter of this process on Linux.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ProcessMeterLinuxAdapter;

impl ProcessMeterPort for ProcessMeterLinuxAdapter {
    fn cpu(&self) -> Result<CpuTime, Error> {
        Ok(CpuTime(cpu_time::ProcessTime::now().as_duration()))
    }
    fn reset_peak(&self) -> Result<(), Error> {
        std::fs::write(constants::CLEAR_REFS, constants::RESET_PEAK)?;
        Ok(())
    }
    fn peak(&self) -> Result<ResidentBytes, Error> {
        let status = std::fs::read_to_string(constants::STATUS)?;
        let kibibytes = status
            .lines()
            .find_map(|line| line.strip_prefix(constants::PEAK_LABEL)?.split_whitespace().next())
            .ok_or_else(|| Error::Invalid(constants::NO_PEAK.into()))?
            .parse::<u64>()
            .map_err(|error| Error::Invalid(format!("{}: {error}", constants::NO_PEAK)))?;
        kibibytes
            .checked_mul(constants::KIBIBYTE)
            .map(ResidentBytes)
            .ok_or_else(|| Error::Invalid(constants::NO_PEAK.into()))
    }
}
