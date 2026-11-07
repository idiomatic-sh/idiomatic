//! Builds the job scheduling adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The rayon job scheduling adapter: it runs the jobs on the rayon pool its caller runs on.
#[must_use]
pub(crate) fn rayon() -> impl crate::outbound::job_scheduling::interface::JobSchedulingPort {
    super::adapters::rayon::adapter::JobSchedulingRayonAdapter
}
