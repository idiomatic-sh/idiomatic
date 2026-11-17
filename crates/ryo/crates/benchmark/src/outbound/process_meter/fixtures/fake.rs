//! A meter that answers from a script: the CPU time each read advances by and the peak each read answers.
use crate::outbound::process_meter::interface::ProcessMeterPort;
use crate::outbound::process_meter::types::CpuTime;
use ryo_benchmark_contracts::domain::measurements::types::ResidentBytes;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Mutex;

/// The bytes of the block the contract's reset row touches and frees before the reset.
pub(crate) const FREED: usize = 64 << 20;

/// What the fake meter answers.
#[derive(Debug)]
pub struct ProcessMeterFake {
    /// The CPU time each `cpu` read advances the clock by, in order; the last repeats.
    cpu_steps: Vec<std::time::Duration>,
    /// The peak each `peak` read answers, in order; the last repeats.
    peaks: Vec<ResidentBytes>,
    /// What was answered so far: the clock, the CPU reads, the peak reads and the resets.
    state: Mutex<Reads>,
}

/// What the fake meter has answered so far.
#[derive(Clone, Copy, Debug, Default)]
struct Reads {
    /// The CPU time so far.
    clock: std::time::Duration,
    /// How many CPU reads were answered.
    cpu_reads: usize,
    /// How many peak reads were answered.
    peak_reads: usize,
    /// How many resets were asked for.
    resets: usize,
}

impl ProcessMeterFake {
    /// A meter whose clock advances by `cpu_steps` and whose peaks are `peaks`.
    #[must_use]
    pub fn new(cpu_steps: Vec<std::time::Duration>, peaks: Vec<ResidentBytes>) -> Self {
        Self {
            cpu_steps,
            peaks,
            state: Mutex::new(Reads::default()),
        }
    }
}

impl Default for ProcessMeterFake {
    /// A clock that advances by a millisecond each read and a peak of one mebibyte.
    fn default() -> Self {
        Self::new(
            vec![std::time::Duration::from_millis(1)],
            vec![ResidentBytes(1 << 20)],
        )
    }
}

impl ProcessMeterPort for ProcessMeterFake {
    fn cpu(&self) -> Result<CpuTime, Error> {
        let mut state = self.state.lock().map_err(|_poisoned| {
            Error::Invalid(String::from("the fake meter's lock is poisoned"))
        })?;
        let step = self
            .cpu_steps
            .get(state.cpu_reads)
            .or_else(|| self.cpu_steps.last())
            .copied()
            .unwrap_or_default();
        state.cpu_reads += 1;
        state.clock += step;
        Ok(CpuTime(state.clock))
    }
    fn reset_peak(&self) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_poisoned| {
            Error::Invalid(String::from("the fake meter's lock is poisoned"))
        })?;
        state.resets += 1;
        Ok(())
    }
    fn peak(&self) -> Result<ResidentBytes, Error> {
        let mut state = self.state.lock().map_err(|_poisoned| {
            Error::Invalid(String::from("the fake meter's lock is poisoned"))
        })?;
        let peak = self
            .peaks
            .get(state.peak_reads)
            .or_else(|| self.peaks.last())
            .copied()
            .unwrap_or(ResidentBytes(0));
        state.peak_reads += 1;
        Ok(peak)
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
