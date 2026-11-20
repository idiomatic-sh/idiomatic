//! The values `load` takes.

use crate::domain::lowering::types::{RefusedRule, SelectedRules};
use crate::domain::packs::types::{PackBytes, PackDigest};
use crate::domain::programs::types::Programs;
use crate::domain::statistics::types::StatisticsBytes;

/// What programs are made from.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::SelectedRules;
/// use ryo_engine::domain::statistics::types::StatisticsBytes;
/// use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
/// use ryo_engine::inbound::use_cases::load::types::LoadSource;
///
/// let source = LoadSource::Pack(PackBytes(b""), PackDigest([0; 32]));
/// assert!(matches!(source, LoadSource::Pack(..)));
/// assert!(matches!(LoadSource::Plans(SelectedRules(&[]), StatisticsBytes(&[])), LoadSource::Plans(..)));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum LoadSource<'a> {
    /// The selected rules, each with its compiled plan and what its parameters are bound to, in rule order; and the statistics
    /// of a sample of the same rules, or no bytes.
    Plans(SelectedRules<'a>, StatisticsBytes<'a>),
    /// A compiled pack and the digest it must have been written for.
    Pack(PackBytes<'a>, PackDigest),
}

/// What a load answers: the programs of the rules it accepted, and each rule it refused with why.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::accepted;
/// use ryo_engine::inbound::use_cases::load::types::LoadedRules;
///
/// let loaded = LoadedRules::default();
/// assert!(accepted(&loaded.programs).is_empty() && loaded.refused.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct LoadedRules {
    /// The programs of the accepted rules.
    pub programs: Programs,
    /// The refused rules, in rule order.
    pub refused: Vec<RefusedRule>,
}

/// Which lowering of a load a pack is of: a load lowers its rules once, and once more without the rules an encoding refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoadPass {
    /// The first: a rule the encoding refuses is taken out and the others are lowered once more.
    First,
    /// The last: there is no other, so a rule the encoding refuses takes every rule of the pack with it.
    Last,
}
