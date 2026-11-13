//! The job scheduling port, implemented with rayon: the jobs are spread over the workers of the pool the caller runs on, the one the engine
//! installs around the language's work, or rayon's global pool outside one. No pool is built here.

use rayon::prelude::*;

use crate::outbound::job_scheduling::interface::JobSchedulingPort;

/// The rayon job scheduling adapter.
#[derive(Debug)]
pub(crate) struct JobSchedulingRayonAdapter;

impl JobSchedulingPort for JobSchedulingRayonAdapter {
    fn run(&self, jobs: &mut [Box<dyn FnMut() + Send + '_>]) {
        jobs.par_iter_mut().for_each(|job| job());
    }
}
