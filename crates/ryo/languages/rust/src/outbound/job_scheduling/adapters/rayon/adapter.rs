//! The job scheduling port, implemented with rayon: the jobs are spread over the workers of the pool the caller runs on, the one the engine
//! installs around the language's work, or rayon's global pool outside one. No pool is built here.

use rayon::prelude::*;

use crate::outbound::job_scheduling::interface::JobSchedulingPort;

/// The rayon job scheduling adapter.
#[derive(Debug)]
pub(crate) struct JobSchedulingRayonAdapter;

impl JobSchedulingPort for JobSchedulingRayonAdapter {
    fn run(&self, jobs: crate::outbound::job_scheduling::types::Jobs<'_>) {
        jobs.0.into_par_iter().for_each(|job| job());
    }
}
