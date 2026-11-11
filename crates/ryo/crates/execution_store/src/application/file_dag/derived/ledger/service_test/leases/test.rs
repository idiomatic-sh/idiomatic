//! Shared, required and optional leases of one ledger: what each charges, previews and releases.
use super::fixtures::{
    Observed, OptionalRelease, OptionalRetention, PositionCapacity, ReleasePreview, ReusedKey,
    SharedAlias, optional_release, optional_retention, position_capacity, release_preview,
    reused_key, shared_alias,
};
use crate::application::file_dag::derived::ledger::types::{ByteCount, Credit, UnitCount};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;

/// The leases a case takes and releases.
#[derive(Clone, Copy, Debug)]
enum Leased {
    /// One 100-byte backing listed twice in a required lease that two consumers share.
    SharedAlias,
    /// A required lease of 100 bytes, then an optional one of the same backing.
    OptionalRetention,
    /// An optional view and a required lease of 400 bytes and 100 positions; the view goes first.
    OptionalRelease,
    /// A required lease of a 100-position backing listed twice, then two optional leases of it.
    PositionCapacity,
    /// Two optional handles of one backing, a live consumer, a required lease, then a shared handle.
    ReleasePreview,
    /// A required lease of 10 bytes at key 3, then six asks that give a key a second size.
    ReusedKey,
}

type Input = Leased;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("shared_alias_is_charged_once_and_last_consumer_releases_it"),
        word: Word::Succeeds,
        input: || Leased::SharedAlias,
        expect: Expect::Value(Observed::SharedAlias(SharedAlias {
            required: ByteCount(100),
            unique: ByteCount(100),
            unique_after_first_release: ByteCount(100),
            required_after: ByteCount(0),
            unique_after: ByteCount(0),
            peak_unique: ByteCount(100),
        })),
    },
    Case {
        name: CaseName(
            "optional_retention_survives_required_release_without_double_physical_charge",
        ),
        word: Word::Succeeds,
        input: || Leased::OptionalRetention,
        expect: Expect::Value(Observed::OptionalRetention(OptionalRetention {
            incremental_before: ByteCount(100),
            both: (
                ByteCount(100),
                ByteCount(100),
                ByteCount(100),
                ByteCount(100),
            ),
            incremental_beside: ByteCount(0),
            retained: (ByteCount(0), ByteCount(100), ByteCount(0), ByteCount(100)),
            unique_after: ByteCount(0),
        })),
    },
    Case {
        name: CaseName("optional_release_keeps_required_backing_and_recharges_the_view_allowance"),
        word: Word::Succeeds,
        input: || Leased::OptionalRelease,
        expect: Expect::Value(Observed::OptionalRelease(OptionalRelease {
            both: (
                ByteCount(400),
                ByteCount(400),
                ByteCount(400),
                ByteCount(400),
            ),
            required_only: (
                ByteCount(400),
                ByteCount(0),
                UnitCount(0),
                ByteCount(0),
                ByteCount(400),
            ),
            incremental_bytes: ByteCount(400),
            incremental_units: UnitCount(100),
            unique_after: ByteCount(0),
            peak_required: ByteCount(400),
            peak_optional: ByteCount(400),
            peak_view_optional: ByteCount(400),
            peak_overlap: ByteCount(400),
            peak_optional_units: UnitCount(100),
        })),
    },
    Case {
        name: CaseName(
            "position_capacity_is_charged_once_per_optional_backing_and_survives_required_release",
        ),
        word: Word::Succeeds,
        input: || Leased::PositionCapacity,
        expect: Expect::Value(Observed::PositionCapacity(PositionCapacity {
            units_while_required: UnitCount(0),
            incremental_before: UnitCount(100),
            units_with_optionals: UnitCount(100),
            incremental_beside: UnitCount(0),
            units_after_required: UnitCount(100),
            units_after_one_optional: UnitCount(100),
            units_after: UnitCount(0),
            peak_units: UnitCount(100),
        })),
    },
    Case {
        name: CaseName(
            "optional_release_preview_requires_every_last_handle_and_counts_units_separately",
        ),
        word: Word::Succeeds,
        input: || Leased::ReleasePreview,
        expect: Expect::Value(Observed::ReleasePreview(ReleasePreview {
            first_alone: Some(Credit {
                bytes: ByteCount(0),
                units: UnitCount(0),
            }),
            every_handle: Some(Credit {
                bytes: ByteCount(400),
                units: UnitCount(100),
            }),
            while_consumed: Some(Credit {
                bytes: ByteCount(0),
                units: UnitCount(0),
            }),
            beside_required: Some(Credit {
                bytes: ByteCount(0),
                units: UnitCount(100),
            }),
            shared_alone: Some(Credit {
                bytes: ByteCount(0),
                units: UnitCount(0),
            }),
            shared_with_its_other_slot: Some(Credit {
                bytes: ByteCount(250),
                units: UnitCount(60),
            }),
            units_after: UnitCount(0),
        })),
    },
    Case {
        name: CaseName("a_reused_key_cannot_change_its_immutable_size"),
        word: Word::Succeeds,
        input: || Leased::ReusedKey,
        expect: Expect::Value(Observed::ReusedKey(ReusedKey {
            refusals: [
                Some(OperatingError::ShapeMismatch),
                Some(OperatingError::ShapeMismatch),
                Some(OperatingError::ShapeMismatch),
                Some(OperatingError::ShapeMismatch),
                Some(OperatingError::ShapeMismatch),
                Some(OperatingError::ShapeMismatch),
            ],
            unchanged: true,
        })),
    },
];

/// Takes and releases the input's leases and answers what the ledger showed.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Leased::SharedAlias => Observed::SharedAlias(shared_alias()?),
        Leased::OptionalRetention => Observed::OptionalRetention(optional_retention()?),
        Leased::OptionalRelease => Observed::OptionalRelease(optional_release()?),
        Leased::PositionCapacity => Observed::PositionCapacity(position_capacity()?),
        Leased::ReleasePreview => Observed::ReleasePreview(release_preview()?),
        Leased::ReusedKey => Observed::ReusedKey(reused_key()?),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
