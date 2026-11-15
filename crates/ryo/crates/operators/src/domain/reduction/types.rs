//! Borrowed reduction contracts; no engine authority.
/// All text is borrowed from engine-acquired backing, including authored literal lists.
#[derive(Clone, Copy, Debug)]
pub enum DigestDatum<'a> {
    Unlocated,
    Text(ryo_values::domain::row_texts::types::CellText<'a>),
    Number(ryo_values::domain::plan_parts::types::WholeNumber),
    Flag(ryo_values::domain::plan_parts::types::FlagState),
    Numbers(&'a [ryo_values::domain::plan_parts::types::WholeNumber]),
    Texts(&'a [ryo_values::domain::plan_parts::types::LiteralText]),
}

/// The folded digest a group finishes with; zero for a counted group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupDigest(pub i64);

/// The stable hash of one resolved value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValueHash(pub u64);

/// The engine-owned scratch the distinct value hashes of one group are collected in.
pub struct HashScratch<'a>(pub &'a mut Vec<u64>);

/// The hashes of a group's values a worker keeps between groups, for a group digest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GroupHashes(pub(crate) Vec<u64>);

/// What a group digest folds a group's value hashes to: how many distinct values, and their digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FoldedDigest {
    /// How many distinct values.
    pub(crate) counted: ryo_values::domain::rows::types::RowCount,
    /// Their digest.
    pub(crate) digest: GroupDigest,
}

