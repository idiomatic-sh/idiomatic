//! The cases of `file_execution`, by group.
mod closure_abort;
#[cfg(test)]
mod distinct_integration;
mod execution_boundary;
#[cfg(test)]
mod fixtures;
mod independent_roots;
mod join;
mod local_caches;
mod measured_work;
mod operation_routes;
mod ordered_dfs;
mod ordered_partial;
#[cfg(test)]
mod preparation;
mod presence;
mod producer_lifetime;
#[cfg(test)]
mod readiness;
mod replay;
mod route_controls;
mod scratch_owner;
#[cfg(any(test, feature = "fixtures"))]
mod walk_routes;
