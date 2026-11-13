//! The first argument of a measure invocation and the scope it holds the run to.
use ryo_benchmark_contracts::domain::requests::constants::{MEASURE, MEASURE_ACCOUNTED_VALID};
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;

/// The first argument of a measure invocation.
#[derive(Clone, Copy, Debug)]
pub struct MeasureCommand<'a>(pub &'a str);

impl MeasureCommand<'_> {
    /// Whether the command holds the run to the accounted inventory or only to the rules it selects.
    ///
    /// # Examples
    /// ```
    /// use ryo_benchmark::domain::command_line::types::measure_command::value::MeasureCommand;
    /// use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;
    /// assert_eq!(MeasureCommand("measure").scope(), BenchmarkCoverage::Accounted);
    /// ```
    #[must_use]
    pub fn scope(self) -> BenchmarkCoverage {
        if matches!(self.0, MEASURE | MEASURE_ACCOUNTED_VALID) {
            BenchmarkCoverage::Accounted
        } else {
            BenchmarkCoverage::Interim
        }
    }
}
