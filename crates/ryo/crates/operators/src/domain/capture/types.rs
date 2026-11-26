//! Borrowed capture contracts; no engine authority.
/// Engine-owned symbol collection for one complete path-capture match.
pub trait CaptureOutput {
    /// Receives the ranges of the slots of one complete match, in slot order.
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



