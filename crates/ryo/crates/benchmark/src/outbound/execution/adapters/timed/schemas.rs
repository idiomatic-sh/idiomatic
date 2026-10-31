//! GNU time's deliberately separate, locale-independent resource record.
use crate::domain::{diagnostics::errors::Error, measurements::types::Resources};
use std::time::Duration;

/// One complete locale-independent GNU time resource record.
pub(super) struct ResourcesSchema<'a>(pub(super) &'a str);

pub(super) fn decode(record: &str) -> Result<Resources, Error> {
    ResourcesSchema(record).try_into()
}

impl TryFrom<ResourcesSchema<'_>> for Resources {
    type Error = Error;
    fn try_from(ResourcesSchema(record): ResourcesSchema<'_>) -> Result<Self, Error> {
        let fields: Vec<_> = record.trim().split('\t').collect();
        let [user, system, resident] = fields.as_slice() else {
            return Err(Error::Invalid("invalid GNU time resource record".into()));
        };
        let seconds = |value: &str| -> Result<Duration, Error> {
            let seconds = value
                .parse::<f64>()
                .map_err(|_| Error::Invalid("invalid child CPU seconds".into()))?;
            Duration::try_from_secs_f64(seconds)
                .map_err(|_| Error::Invalid("invalid child CPU seconds".into()))
        };
        let cpu = seconds(user)?
            .checked_add(seconds(system)?)
            .ok_or_else(|| Error::Invalid("child CPU overflow".into()))?;
        let peak_rss_bytes = resident
            .parse::<u64>()
            .map_err(|_| Error::Invalid("invalid child peak RSS".into()))?
            .checked_mul(1024)
            .ok_or_else(|| Error::Invalid("invalid child peak RSS".into()))?;
        Ok(Resources {
            cpu,
            peak_rss_bytes,
        })
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
