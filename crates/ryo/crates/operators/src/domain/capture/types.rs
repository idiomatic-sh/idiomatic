//! Borrowed capture contracts; no engine authority.
/// Engine-owned symbol collection for one complete path-capture match.
pub trait CaptureOutput {
    fn captures(&mut self, ranges: CaptureSpans<'_>);
}

/// The authored path pattern, with `{name}` and `{*name}` slots.
#[derive(Clone, Copy, Debug)]
pub struct SlotPattern<'a>(pub &'a str);

/// A path, or the part of it not yet matched.
#[derive(Clone, Copy, Debug)]
pub struct SlotSubject<'a>(pub &'a str);

/// The engine-owned ranges of the slots matched so far.
#[derive(Debug)]
pub struct CaptureSlots<'a>(pub &'a mut Vec<std::ops::Range<usize>>);

/// How many slots a path pattern has: the columns a capture appends to each row.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::capture::types::CaptureSlotCount;
/// assert_eq!(CaptureSlotCount(2).0, 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureSlotCount(pub usize);

/// The ranges of every slot of one complete match, in slot order.
#[derive(Clone, Copy, Debug)]
pub struct CaptureSpans<'a>(pub &'a [std::ops::Range<usize>]);

/// What a capture lowers to: the path column and the slot pattern.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CaptureParts {
    /// The column holding the path.
    pub(crate) path: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The pattern, with its slots.
    pub(crate) pattern: ryo_values::domain::plan_parts::types::CapturePattern,
}

/// A capture's slot places, one way after another, kept to be minted once the match is done.
pub(crate) struct SlotPlaces<'a> {
    /// The place of every slot of every way so far, in way order.
    pub(crate) places: &'a mut crate::domain::contract::types::RangeScratch,
    /// How many ways the pattern matched.
    pub(crate) ways: ryo_relations::domain::relations::types::Count,
}

impl CaptureOutput for SlotPlaces<'_> {
    fn captures(&mut self, ranges: CaptureSpans<'_>) {
        self.places.0.extend_from_slice(ranges.0);
        self.ways.0 += 1;
    }
}
