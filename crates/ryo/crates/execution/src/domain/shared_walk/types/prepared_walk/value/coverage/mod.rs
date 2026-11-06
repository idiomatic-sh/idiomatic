//! Independent physical-head and per-operation route coverage over the relation catalog.
#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
#[cfg(test)]
mod heads;
#[cfg(test)]
mod routes;
