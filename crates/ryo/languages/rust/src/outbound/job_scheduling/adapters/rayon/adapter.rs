//! The job scheduling port, implemented with rayon: the jobs are spread over the workers of the pool the caller runs on, the one the engine
//! installs around the language's work, or rayon's global pool outside one. No pool is built here.

use rayon::prelude::*;

use crate::outbound::job_scheduling::interface::JobSchedulingPort;
use crate::outbound::job_scheduling::types::{JobIndex, ScheduledJobCount};

/// The rayon job scheduling adapter.
#[derive(Debug)]
pub(crate) struct JobSchedulingRayonAdapter;

impl JobSchedulingPort for JobSchedulingRayonAdapter {
    fn run(&self, count: ScheduledJobCount, job: &(dyn Fn(JobIndex) + Sync)) {
        (0..count.0)
            .into_par_iter()
            .for_each(|index| job(JobIndex(index)));
    }
}
