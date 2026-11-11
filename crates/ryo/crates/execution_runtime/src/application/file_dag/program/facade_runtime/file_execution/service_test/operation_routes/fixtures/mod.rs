//! The sources and chain runner every operation route is proved through.
pub(super) mod chain;
pub(super) mod sources;
use self::chain::Probe;
pub(crate) fn describe(probe: &Probe) -> String {
    match &probe.outcome {
        Ok((table, _)) => format!("ok {:?} {:?} {}", table.kinds, table.cells, table.count.0),
        Err(error) => format!("err {error:?}"),
    }
}
