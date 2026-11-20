//! The values `load` takes.

use crate::domain::lowering::types::SelectedRules;
use crate::domain::packs::types::{PackBytes, PackDigest};

/// What programs are made from.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
/// use ryo_engine::inbound::use_cases::load::types::LoadSource;
///
/// let source = LoadSource::Pack(PackBytes(b""), PackDigest([0; 32]));
/// assert!(matches!(source, LoadSource::Pack(..)));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum LoadSource<'a> {
    /// The selected rules and their compiled plans, in rule order.
    Plans(SelectedRules<'a>),
    /// A compiled pack and the digest it must have been written for.
    Pack(PackBytes<'a>, PackDigest),
}
