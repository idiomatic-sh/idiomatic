//! The machine the SDK runs on, which a caller's request for workers is cut to.

/// How many threads the machine can run at once, which the caller's request for workers is cut to.
pub(crate) fn machine_workers() -> ryo_values::domain::runs::types::MachineWorkers {
    ryo_values::domain::runs::types::MachineWorkers(
        u32::try_from(std::thread::available_parallelism().map_or(1, std::num::NonZero::get))
            .unwrap_or(1),
    )
}
