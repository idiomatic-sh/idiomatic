//! The bmf port, implemented with `serde_json`: a tool's Bencher Metric Format read through the model of it.

use super::constants::NOTHING;
use super::schemas::read;
use crate::outbound::bmf::errors::BmfError;
use crate::outbound::bmf::interface::BmfPort;
use crate::outbound::bmf::types::{BmfText, Measurements};

/// The serde-json adapter.
#[derive(Debug)]
pub(crate) struct BmfJsonAdapter;

impl BmfPort for BmfJsonAdapter {
    fn read(&self, text: &BmfText) -> Result<Measurements, BmfError> {
        if text.0.trim().is_empty() {
            return Err(BmfError::Unparsable {
                reason: NOTHING.to_owned(),
            });
        }
        read(&text.0).map_err(|reason| BmfError::Unparsable { reason })
    }
}
